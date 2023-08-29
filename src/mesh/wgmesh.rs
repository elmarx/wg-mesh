use crate::error;
use crate::mesh::filter_peers::filter_peers;
use crate::traits::{NodeRepository, RoutingService, Wireguard};

pub struct WgMesh<PeerRepositoryT, RoutingServiceT, WireguardT>
where
    PeerRepositoryT: NodeRepository,
    RoutingServiceT: RoutingService,
    WireguardT: Wireguard,
{
    node_repository: PeerRepositoryT,
    routing_service: RoutingServiceT,
    wireguard: WireguardT,
}

impl<NodeRepositoryT, RoutingServiceT, WireguardT>
    WgMesh<NodeRepositoryT, RoutingServiceT, WireguardT>
where
    NodeRepositoryT: NodeRepository,
    RoutingServiceT: RoutingService,
    WireguardT: Wireguard,
{
    pub fn new(
        node_repository: NodeRepositoryT,
        routing_service: RoutingServiceT,
        wireguard: WireguardT,
    ) -> WgMesh<NodeRepositoryT, RoutingServiceT, WireguardT> {
        WgMesh {
            node_repository,
            routing_service,
            wireguard,
        }
    }

    pub async fn execute(self, mesh_record: &str) -> Result<(), error::WgMesh> {
        let interface_pubkey = self.wireguard.get_interface_pubkey()?;

        // first, get a list of all Peers belonging to the mesh
        let peers = self.node_repository.fetch_all_peers(mesh_record).await?;

        // get the list of peers we actually want to peer with
        let relevant_peers = filter_peers(&interface_pubkey, peers)?;

        // and add them to wireguard
        // TODO: get list of current peers and remove, update, or add peers
        self.wireguard.replace_peers(relevant_peers.as_slice())?;

        // …as well as adding direct routes for the "allowed ips"
        // TODO: get list of current routes and remove, update, or add routes
        self.routing_service
            .add_routes(relevant_peers.as_slice())
            .await?;

        Ok(())
    }
}
