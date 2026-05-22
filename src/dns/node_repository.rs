use std::net::IpAddr;
use std::str::from_utf8;

use crate::error;
use crate::error::NodeRepository::InvalidNameserver;
use crate::error::NodeRepository::MissingPubkeyRecord;
use async_trait::async_trait;
use futures::future::join_all;
use hickory_resolver::TokioResolver;
use hickory_resolver::config::{NameServerConfig, ResolverConfig};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::proto::rr::RData;

use crate::model::Peer;
use crate::traits::NodeRepository;

pub struct DnsNodeRepository {
    resolver: TokioResolver,
}

impl DnsNodeRepository {
    pub fn new(address: Option<String>) -> Result<Self, error::NodeRepository> {
        let resolver = match address {
            Some(a) => {
                let nameserver = a
                    .parse::<IpAddr>()
                    .map_err(|_| InvalidNameserver(a.clone()))?;
                let nameserver = NameServerConfig::udp_and_tcp(nameserver);

                let config = ResolverConfig::from_parts(None, vec![], vec![nameserver]);

                TokioResolver::builder_with_config(config, TokioRuntimeProvider::default())
                    .build()
                    .expect("failed to create DNS resolver")
            }
            None => TokioResolver::builder_tokio()?.build()?,
        };

        Ok(DnsNodeRepository { resolver })
    }
}

#[async_trait(?Send)]
impl NodeRepository for DnsNodeRepository {
    async fn list_mesh_nodes(
        &self,
        mesh_record: &str,
    ) -> Result<Vec<String>, error::NodeRepository> {
        let response = self.resolver.txt_lookup(mesh_record).await?;

        Ok(response
            .answers()
            .iter()
            .filter_map(|r| {
                if let RData::TXT(txt) = &r.data {
                    Some(
                        String::from_utf8(txt.txt_data.concat())
                            .expect("non-UTF-8 TXT record in mesh-record"),
                    )
                } else {
                    None
                }
            })
            .collect())
    }

    async fn fetch_peer(&self, node_addr: &str) -> Result<Peer, error::NodeRepository> {
        let qname = format!("_wireguard.{node_addr}");

        let has_public_ipv4_address = match self.resolver.ipv4_lookup(node_addr).await {
            Ok(lookup) => Ok(lookup
                .answers()
                .iter()
                .filter_map(|r| {
                    if let RData::A(a) = &r.data {
                        Some(a.0)
                    } else {
                        None
                    }
                })
                .any(|addr| !addr.is_private())),
            Err(e) if e.is_no_records_found() => Ok(false),
            Err(e) => Err(e),
        }?;

        let pubkey_response = self.resolver.txt_lookup(&qname).await?;
        let pubkey_record = pubkey_response
            .answers()
            .iter()
            .find_map(|r| {
                if let RData::TXT(txt) = &r.data {
                    Some(txt)
                } else {
                    None
                }
            })
            .ok_or_else(|| MissingPubkeyRecord(qname.clone()))?;
        let pubkey_bytes = pubkey_record.txt_data.concat();
        let pubkey = from_utf8(&pubkey_bytes).expect("non-UTF-8 TXT record for peer pubkey");

        let allowed_ips_response = self.resolver.ipv4_lookup(&qname).await?;
        let allowed_ips: Vec<_> = allowed_ips_response
            .answers()
            .iter()
            .filter_map(|r| {
                if let RData::A(a) = &r.data {
                    Some(a.0)
                } else {
                    None
                }
            })
            .map(|addr| format!("{addr}/32"))
            .collect();

        let ttl = pubkey_response
            .answers()
            .iter()
            .chain(allowed_ips_response.answers().iter())
            .map(|r| r.ttl)
            .min()
            .unwrap_or(300);
        let ttl = std::time::Duration::from_secs(ttl.into());

        let site = node_addr
            .chars()
            .skip_while(|c| *c != '.')
            .collect::<String>();

        Ok(Peer {
            public_key: pubkey.to_string(),
            allowed_ips,
            endpoint: (node_addr.into(), 51820),
            site,
            has_public_ipv4_address,
            ttl,
        })
    }

    async fn fetch_all_peers(&self, mesh_record: &str) -> Result<Vec<Peer>, error::NodeRepository> {
        let all_peers: Result<Vec<_>, _> = join_all(
            self.list_mesh_nodes(mesh_record)
                .await?
                .iter()
                .map(|p| self.fetch_peer(p)),
        )
        .await
        .into_iter()
        .collect();

        all_peers
    }
}
