use std::net::ToSocketAddrs;
use std::str::from_utf8;

use crate::error;
use crate::error::NodeRepository::{
    InvalidNameserver, MissingPubkeyRecord, UnresolvableSocketAddress,
};
use async_trait::async_trait;
use futures::future::join_all;
use rsdns::clients::tokio::Client;
use rsdns::clients::ClientConfig;
use rsdns::constants::Class;
use rsdns::records::data::{Txt, A};
use rsdns::Error;

use crate::model::Peer;
use crate::traits::NodeRepository;

pub struct DnsNodeRepository {
    config: ClientConfig,
}

impl DnsNodeRepository {
    pub fn from_address(address: &str) -> Result<Self, error::NodeRepository> {
        let nameserver = address
            .to_socket_addrs()
            .map_err(|e| UnresolvableSocketAddress(e, address.to_string()))?
            .next()
            .ok_or_else(|| InvalidNameserver(address.to_string()))?;

        let config = ClientConfig::with_nameserver(nameserver);

        Ok(DnsNodeRepository { config })
    }
}

#[async_trait(?Send)]
impl NodeRepository for DnsNodeRepository {
    async fn list_mesh_nodes(
        &self,
        mesh_record: &str,
    ) -> Result<Vec<String>, error::NodeRepository> {
        let mut client = Client::new(self.config.clone()).await?;

        let response = client.query_rrset::<Txt>(mesh_record, Class::In).await?;

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

        let has_public_ipv4_address = match client.query_rrset::<A>(node_addr, Class::In).await {
            Ok(a_query) => Ok(a_query.rdata.iter().any(|a| !a.address.is_private())),
            Err(Error::NoAnswer) => Ok(false),
            Err(e) => Err(e),
        }?;

        let pubkey_query = client.query_rrset::<Txt>(&qname, Class::In).await?;
        let pubkey = from_utf8(
            &pubkey_query
                .rdata
                .first()
                .ok_or_else(|| MissingPubkeyRecord(qname.clone()))?
                .text,
        )
        .expect("non-UTF-8 TXT record for peer pubkey");
        let allowed_ips_query = client.query_rrset::<A>(&qname, Class::In).await?;
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
