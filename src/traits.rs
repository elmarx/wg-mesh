use async_trait::async_trait;

use crate::error::WgMesh as WgMeshError;
use crate::model::Peer;

#[async_trait(?Send)]
pub trait NodeRepository {
    /// return the list of all nodes for this mesh
    async fn list_mesh_nodes(&self, mesh_record: &str) -> Vec<String>;

    /// given a node address, resolve all parameters into a Peer struct
    async fn fetch_peer(&self, peer_addr: &str) -> Result<Peer, WgMeshError>;

    /// fetch all Peers for this mesh
    async fn fetch_all_peers(&self, mesh_record: &str) -> Vec<Peer>;
}

#[async_trait(?Send)]
pub trait RoutingService {
    async fn add_routes(&self, peers: &[Peer]);
}

pub trait Wireguard {
    fn replace_peers(&self, peers: &[Peer]);
    fn get_interface_pubkey(&self) -> Result<String, WgMeshError>;
}
