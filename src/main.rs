use std::env::args;

use rtnetlink::new_connection;

use crate::dns::node_repository::DnsNodeRepository;
use crate::mesh::wgmesh::WgMesh;
use crate::routing::routing_service::RoutingServiceImpl;
use crate::wireguard::wireguard_device::WireguardImpl;

mod dns;
mod error;
mod mesh;
mod model;
mod routing;
mod traits;
mod wireguard;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: use clap for proper input/argument parsing
    let interface_name = args().nth(2).unwrap_or("wg0".to_string());
    let mesh_record = args()
        .nth(1)
        .expect("please pass record name with the peer list");

    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // TODO: parse address(es) from resolve.conf
    let peer_repository = DnsNodeRepository::from_address("dns.quad9.net:53")?;

    // TODO: loop/wait for device to be available
    let wireguard_device = WireguardImpl::new(&interface_name)?;
    let routing_service = RoutingServiceImpl::new(handle, &interface_name);

    let wg_mesh = WgMesh::new(peer_repository, routing_service, wireguard_device);
    // TODO: loop/re-execute
    wg_mesh.execute(&mesh_record).await?;

    Ok(())
}
