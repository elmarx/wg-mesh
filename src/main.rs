use error::WgMesh as WgMeshError;
use futures::future::join_all;
use model::Peer;
use rsdns::clients::tokio::Client;
use rsdns::clients::ClientConfig;
use rsdns::records::data::Txt;
use rsdns::{constants::Class, records::data::A, Error};
use std::env::args;
use std::str::{from_utf8, FromStr};
use wireguard_control::Backend;
use wireguard_control::Device;
use wireguard_control::DeviceUpdate;
use wireguard_control::InterfaceName;
use wireguard_control::PeerConfigBuilder;

mod error;
mod model;

#[cfg(target_os = "linux")]
const BACKEND: Backend = Backend::Kernel;

async fn get_peer(peer_addr: &str) -> Peer {
    let config = ClientConfig::with_nameserver("[2620:fe::9]:53".parse().unwrap());
    let mut client = Client::new(config).await.unwrap();

    let qname = format!("_wireguard.{peer_addr}");

    let has_public_ipv4_address = match client.query_rrset::<A>(peer_addr, Class::In).await {
        Ok(a_query) => Ok(a_query.rdata.iter().any(|a| !a.address.is_private())),
        Err(Error::NoAnswer) => Ok(false),
        Err(e) => Err(e),
    }
    .unwrap();

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

    Peer {
        public_key: pubkey.to_string(),
        allowed_ips,
        endpoint: (peer_addr.into(), 51820),
        site,
        has_public_ipv4_address,
    }
}

#[tokio::main]
async fn main() -> Result<(), WgMeshError> {
    let mesh_record = args()
        .nth(1)
        .expect("please pass record name with the peer list");

    let config = ClientConfig::with_nameserver("[2620:fe::fe]:53".parse().unwrap());
    let mut client = Client::new(config).await.unwrap();
    let response = client
        .query_rrset::<Txt>(&mesh_record, Class::In)
        .await
        .unwrap();

    let peers = join_all(
        response
            .rdata
            .iter()
            .map(|txt| from_utf8(&txt.text).unwrap())
            .map(get_peer),
    )
    .await;

    let interface_name = args().nth(2).unwrap_or("wg0".to_string());
    let interface_name = InterfaceName::from_str("wg0")
        .map_err(|_| WgMeshError::InvalidInterfaceName(interface_name))?;

    let device = Device::get(&interface_name, BACKEND).map_err(WgMeshError::NoSuchDevice)?;
    let interface_pubkey = device.public_key.ok_or(WgMeshError::NoPubkey)?;
    let interface_pubkey = interface_pubkey.to_base64();

    let this_peer = peers
        .iter()
        .find(|peer| peer.public_key == interface_pubkey)
        .ok_or(WgMeshError::PeerNotPartOfMesh(interface_pubkey.clone()))?;

    let peers = peers
        .iter()
        .filter(|peer| *peer != this_peer)
        .filter(|peer| peer.site != this_peer.site)
        .filter(|peer| !this_peer.has_public_ipv4_address || !peer.has_public_ipv4_address)
        .map(|peer| peer.try_into())
        .collect::<Result<Vec<PeerConfigBuilder>, _>>()?;

    let update = DeviceUpdate::new().replace_peers().add_peers(&peers);

    update
        .apply(&interface_name, BACKEND)
        .map_err(WgMeshError::FailedToApplyConfig)?;

    Ok(())
}
