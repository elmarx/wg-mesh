use crate::error::WgMesh;
use error::WgMesh as WgMeshError;
use futures::future::join_all;
use futures::TryStreamExt;
use ipnet::Ipv4Net;
use model::Peer;
use nix::errno::Errno;
use rsdns::clients::tokio::Client;
use rsdns::clients::ClientConfig;
use rsdns::records::data::Txt;
use rsdns::{constants::Class, records::data::A, Error};
use rtnetlink::new_connection;
use rtnetlink::Error::NetlinkError;
use std::env::args;
use std::net::{SocketAddr, ToSocketAddrs};
use std::str::{from_utf8, FromStr};
use wireguard_control::Backend;
use wireguard_control::Device;
use wireguard_control::DeviceUpdate;
use wireguard_control::InterfaceName;

mod error;
mod model;

#[cfg(target_os = "linux")]
const BACKEND: Backend = Backend::Kernel;

async fn get_peer(nameserver: SocketAddr, peer_addr: &str) -> Result<Peer, WgMeshError> {
    let config = ClientConfig::with_nameserver(nameserver);
    let mut client = Client::new(config).await.unwrap();

    let qname = format!("_wireguard.{peer_addr}");

    let has_public_ipv4_address = match client.query_rrset::<A>(peer_addr, Class::In).await {
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

    let site = peer_addr
        .chars()
        .skip_while(|c| *c != '.')
        .collect::<String>();

    Ok(Peer {
        public_key: pubkey.to_string(),
        allowed_ips,
        endpoint: (peer_addr.into(), 51820),
        site,
        has_public_ipv4_address,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let interface_name = args().nth(2).unwrap_or("wg0".to_string());
    let interface_name = InterfaceName::from_str(&interface_name)
        .map_err(|_| WgMeshError::InvalidInterfaceName(interface_name))?;
    let mesh_record = args()
        .nth(1)
        .expect("please pass record name with the peer list");

    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);

    let route_handle = handle.route();
    let mut link_handle = handle.link();

    let interface = link_handle
        .get()
        .match_name(interface_name.to_string())
        .execute()
        .try_next()
        .await?
        .ok_or("no such interface")?;

    let nameserver = "dns.quad9.net:53"
        .to_socket_addrs()?
        .next()
        .ok_or("could not get socket-address for dns.quad9.net:53")?;
    let config = ClientConfig::with_nameserver(nameserver);
    let mut client = Client::new(config).await?;
    let response = client.query_rrset::<Txt>(&mesh_record, Class::In).await?;

    let peers: Result<Vec<_>, _> = join_all(
        response
            .rdata
            .iter()
            .map(|txt| from_utf8(&txt.text).unwrap())
            .map(|p| get_peer(nameserver, p)),
    )
    .await
    .into_iter()
    .collect();
    let mut peers = peers?;

    let device = Device::get(&interface_name, BACKEND).map_err(WgMeshError::NoSuchDevice)?;
    let interface_pubkey = device.public_key.ok_or(WgMeshError::NoPubkey)?;
    let interface_pubkey = interface_pubkey.to_base64();

    let this_peer = peers
        .iter()
        .position(|peer| peer.public_key == interface_pubkey)
        .ok_or(WgMeshError::PeerNotPartOfMesh(interface_pubkey.clone()))?;
    let this_peer = peers.swap_remove(this_peer);

    let peers: Vec<_> = peers
        .into_iter()
        .filter(|peer| *peer != this_peer)
        .filter(|peer| peer.site != this_peer.site)
        .filter(|peer| !this_peer.has_public_ipv4_address || !peer.has_public_ipv4_address)
        .collect();

    let peer_config_builders = peers
        .iter()
        .map(|peer| peer.try_into())
        .collect::<Result<Vec<_>, _>>()?;

    let update = DeviceUpdate::new()
        .replace_peers()
        .add_peers(&peer_config_builders);

    update
        .apply(&interface_name, BACKEND)
        .map_err(WgMeshError::FailedToApplyConfig)?;

    // TODO: do not always add requests without checks
    let route_add_requests: Vec<_> = peers
        .iter()
        .flat_map(|peer| {
            peer.allowed_ips
                .iter()
                .map(|i| i.parse::<Ipv4Net>().unwrap())
                .map(|ip| {
                    route_handle
                        .add()
                        .v4()
                        .input_interface(interface.header.index)
                        .output_interface(interface.header.index)
                        .destination_prefix(ip.addr(), ip.prefix_len())
                })
        })
        .collect();

    for r in route_add_requests {
        r.execute().await.or_else(|e| -> Result<(), WgMeshError> {
            match e {
                // TODO: this is not very elegant, so better check in the first place if something has to be added or not
                NetlinkError(err) if -err.code == Errno::EEXIST as i32 => Ok(()),
                err => Err(WgMesh::NetlinkError(err)),
            }
        })?;
    }

    Ok(())
}
