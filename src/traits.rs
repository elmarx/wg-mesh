use crate::error;
use async_trait::async_trait;

use crate::model::Peer;

#[async_trait(?Send)]
pub trait NodeRepository {
    /// return the list of all nodes for this mesh
    async fn list_mesh_nodes(
        &self,
        mesh_record: &str,
    ) -> Result<Vec<String>, error::NodeRepository>;

    /// given a node address, resolve all parameters into a Peer struct
    async fn fetch_peer(&self, peer_addr: &str) -> Result<Peer, error::NodeRepository>;

    /// fetch all Peers for this mesh
    async fn fetch_all_peers(&self, mesh_record: &str) -> Result<Vec<Peer>, error::NodeRepository>;
}

#[async_trait(?Send)]
pub trait RoutingService {
    async fn add_routes(&self, peers: &[Peer]) -> Result<(), error::Routing>;
}

pub trait Wireguard {
    fn replace_peers(&self, peers: &[Peer]) -> Result<(), error::Wireguard>;
    fn get_interface_pubkey(&self) -> Result<String, error::Wireguard>;
}
