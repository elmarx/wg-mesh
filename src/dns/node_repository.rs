use std::net::ToSocketAddrs;
use std::str::from_utf8;

use async_trait::async_trait;
use futures::future::join_all;
use rsdns::clients::tokio::Client;
use rsdns::clients::ClientConfig;
use rsdns::constants::Class;
use rsdns::records::data::{Txt, A};
use rsdns::Error;

use crate::error::WgMesh as WgMeshError;
use crate::model::Peer;
use crate::traits::NodeRepository;

pub struct DnsNodeRepository {
    config: ClientConfig,
}

impl DnsNodeRepository {
    pub fn from_address(address: &str) -> Self {
        let nameserver = address
            .to_socket_addrs()
            .unwrap()
            .next()
            .ok_or("could not get socket-address for dns.quad9.net:53")
            .unwrap();

        let config = ClientConfig::with_nameserver(nameserver);

        DnsNodeRepository { config }
    }
}

#[async_trait(?Send)]
impl NodeRepository for DnsNodeRepository {
    async fn list_mesh_nodes(&self, mesh_record: &str) -> Vec<String> {
        let mut client = Client::new(self.config.clone()).await.unwrap();

        let response = client
            .query_rrset::<Txt>(mesh_record, Class::In)
            .await
            .unwrap();

        response
            .rdata
            .iter()
            .map(|txt| from_utf8(&txt.text).unwrap())
            .map(ToString::to_string)
            .collect()
    }

    async fn fetch_peer(&self, node_addr: &str) -> Result<Peer, WgMeshError> {
        let mut client = Client::new(self.config.clone()).await.unwrap();

        let qname = format!("_wireguard.{node_addr}");

        let has_public_ipv4_address = match client.query_rrset::<A>(node_addr, Class::In).await {
            Ok(a_query) => Ok(a_query.rdata.iter().any(|a| !a.address.is_private())),
            Err(Error::NoAnswer) => Ok(false),
            Err(e) => Err(e),
        }
        .map_err(WgMeshError::Rsdns)?;

        let pubkey_query = client.query_rrset::<Txt>(&qname, Class::In).await.unwrap();
        let pubkey = from_utf8(&pubkey_query.rdata.first().unwrap().text).unwrap();
        let allowed_ips_query = client.query_rrset::<A>(&qname, Class::In).await.unwrap();
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

    async fn fetch_all_peers(&self, mesh_record: &str) -> Vec<Peer> {
        let all_peers: Result<Vec<_>, _> = join_all(
            self.list_mesh_nodes(mesh_record)
                .await
                .iter()
                .map(|p| self.fetch_peer(p)),
        )
        .await
        .into_iter()
        .collect();

        all_peers.unwrap()
    }
}
