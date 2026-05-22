use clap::Parser;
use rtnetlink::new_connection;
use tokio::time::sleep;

use crate::cli::Cli;
use crate::dns::node_repository::DnsNodeRepository;
use crate::mesh::wgmesh::WgMesh;
use crate::routing::routing_service::RoutingServiceImpl;
use crate::wireguard::wireguard_device::WireguardImpl;

mod cli;
mod dns;
mod error;
mod mesh;
mod model;
mod routing;
mod traits;
mod wireguard;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    let peer_repository = DnsNodeRepository::new(cli.resolver)?;
    let wireguard_device = WireguardImpl::new(&cli.interface)?;
    let routing_service = RoutingServiceImpl::new(handle, &cli.interface);

    let wg_mesh = WgMesh::new(peer_repository, routing_service, wireguard_device);
    loop {
        let ttl = wg_mesh.execute(&cli.mesh_record).await?;
        sleep(ttl).await;
    }
}
