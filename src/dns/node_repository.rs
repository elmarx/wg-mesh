use std::net::SocketAddr;
use std::str::from_utf8;

use crate::dns::resolver;
use crate::error;
use crate::error::NodeRepository::MissingPubkeyRecord;
use async_trait::async_trait;
use futures::future::join_all;
use rsdns::Error;
use rsdns::clients::ClientConfig;
use rsdns::clients::tokio::Client;
use rsdns::records::Class;
use rsdns::records::data::{A, Txt};

use crate::model::Peer;
use crate::traits::NodeRepository;

pub struct DnsNodeRepository {
    config: ClientConfig,
}

impl DnsNodeRepository {
    pub fn new(resolver: SocketAddr) -> Self {
        let config = ClientConfig::with_nameserver(resolver);

        DnsNodeRepository { config }
    }

    pub fn init(address: Option<String>) -> Result<Self, error::NodeRepository> {
        Ok(match address {
            Some(a) => Self::new(resolver::from_address(&a)?),
            None => Self::new(resolver::from_resolv_conf()),
        })
    }
}

#[async_trait(?Send)]
impl NodeRepository for DnsNodeRepository {
    async fn list_mesh_nodes(
        &self,
        mesh_record: &str,
    ) -> Result<Vec<String>, error::NodeRepository> {
        let mut client = Client::new(self.config.clone()).await?;

        let response = client.query_rrset::<Txt>(mesh_record, Class::IN).await?;

        Ok(response
            .rdata
            .iter()
            .map(|txt| from_utf8(&txt.text).expect("non-UTF-8 TXT record in mesh-record"))
            .map(ToString::to_string)
            .collect())
    }

    async fn fetch_peer(&self, node_addr: &str) -> Result<Peer, error::NodeRepository> {
        let mut client = Client::new(self.config.clone()).await?;

        let qname = format!("_wireguard.{node_addr}");

        let has_public_ipv4_address = match client.query_rrset::<A>(node_addr, Class::IN).await {
            Ok(a_query) => Ok(a_query.rdata.iter().any(|a| !a.address.is_private())),
            Err(Error::NoAnswer) => Ok(false),
            Err(e) => Err(e),
        }?;

        let pubkey_query = client.query_rrset::<Txt>(&qname, Class::IN).await?;
        let pubkey = from_utf8(
            &pubkey_query
                .rdata
                .first()
                .ok_or_else(|| MissingPubkeyRecord(qname.clone()))?
                .text,
        )
        .expect("non-UTF-8 TXT record for peer pubkey");
        let allowed_ips_query = client.query_rrset::<A>(&qname, Class::IN).await?;
        let allowed_ips: Vec<_> = allowed_ips_query
            .rdata
            .iter()
            .map(|a| format!("{}/32", a.address))
            .collect();

        // TODO: "proper" subnet resolving
        // (e.g.: put subnet-mask into DNS?)
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
