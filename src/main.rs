use std::env;
use std::env::args;
use std::time::Duration;

use rtnetlink::new_connection;
use tokio::time::sleep;

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

pub async fn init_retry_wg(interface_name: &str) -> Result<WireguardImpl, error::Wireguard> {
    for _ in 0..12 {
        let wireguard = WireguardImpl::new(interface_name);

        if wireguard.is_ok() {
            return wireguard;
        }

        sleep(Duration::from_secs(5)).await;
    }

    WireguardImpl::new(interface_name)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: use clap for proper input/argument parsing
    let interface_name = args().nth(2).unwrap_or("wg0".to_string());
    let mesh_record = args()
        .nth(1)
        .expect("please pass record name with the peer list");
    let mesh_resolver = env::var("WG_MESH_RESOLVER").ok();

    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    let peer_repository = DnsNodeRepository::init(mesh_resolver)?;

    let wireguard_device = init_retry_wg(&interface_name).await?;
    let routing_service = RoutingServiceImpl::new(handle, &interface_name);

    let wg_mesh = WgMesh::new(peer_repository, routing_service, wireguard_device);
    // TODO: loop/re-execute
    wg_mesh.execute(&mesh_record).await?;

    Ok(())
}
