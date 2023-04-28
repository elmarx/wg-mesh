use rsdns::clients::std::Client;
use rsdns::clients::ClientConfig;
use rsdns::records::data::Txt;
use rsdns::{constants::Class, records::data::A, Error};
use std::env::args;
use std::fmt::Display;
use std::str::from_utf8;

struct Interface {
    listen_port: u16,
    private_key: String,
}

#[derive(Debug, PartialEq, Clone)]
struct Peer {
    public_key: String,
    allowed_ips: Vec<String>,
    endpoint: (String, u16),
    site: String,
    persistent_keepalive: u16,
    has_public_ipv4_address: bool,
}

struct WireguardConfig {
    interface: Interface,
    peers: Vec<Peer>,
}

impl Display for WireguardConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut config = "[Interface]\n".to_string();
        config.push_str(&format!("ListenPort = {}\n", self.interface.listen_port));
        config.push_str(&format!("PrivateKey = {}\n", self.interface.private_key));
        config.push('\n');
        for peer in &self.peers {
            config.push_str("[Peer]\n");
            config.push_str(&format!("PublicKey = {}\n", peer.public_key));
            config.push_str(&format!("AllowedIPs = {}\n", peer.allowed_ips.join(", ")));
            config.push_str(&format!(
                "Endpoint = {}:{}\n",
                peer.endpoint.0, peer.endpoint.1
            ));
            config.push_str(&format!(
                "PersistentKeepalive = {}\n",
                peer.persistent_keepalive
            ));
            config.push('\n');
        }

        write!(f, "{config}")
    }
}

fn get_peer(peer_addr: &str) -> Peer {
    let config = ClientConfig::with_nameserver("[2620:fe::fe]:53".parse().unwrap());
    let mut client = Client::new(config).unwrap();

    let qname = format!("_wireguard.{peer_addr}");

    let has_public_ipv4_address = match client.query_rrset::<A>(peer_addr, Class::In) {
        Ok(a_query) => Ok(a_query.rdata.iter().any(|a| !a.address.is_private())),
        Err(Error::NoAnswer) => Ok(false),
        Err(e) => Err(e),
    }
    .unwrap();

    let pubkey_query = client.query_rrset::<Txt>(&qname, Class::In).unwrap();
    let pubkey = from_utf8(&pubkey_query.rdata.first().unwrap().text).unwrap();
    let allowed_ips_query = client.query_rrset::<A>(&qname, Class::In).unwrap();
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
        persistent_keepalive: 25,
        has_public_ipv4_address,
    }
}

fn main() {
    let config = ClientConfig::with_nameserver("[2620:fe::fe]:53".parse().unwrap());
    let mut client = Client::new(config).unwrap();
    let response = client
        .query_rrset::<Txt>("_wg-mesh.example.com", Class::In)
        .unwrap();

    let peers: Vec<_> = response
        .rdata
        .iter()
        .map(|txt| from_utf8(&txt.text).unwrap())
        .map(get_peer)
        .collect();

    let interface_private_key = args().nth(1).expect("no privatekey given");
    let interface_pubkey = args().nth(2).expect("no pubkey given");

    let interface = Interface {
        listen_port: 51820,
        private_key: interface_private_key,
    };

    let this_peer = peers
        .iter()
        .find(|peer| peer.public_key == interface_pubkey)
        .unwrap();

    let peers = peers
        .iter()
        .filter(|peer| *peer != this_peer)
        .filter(|peer| peer.site != this_peer.site)
        .filter(|peer| !this_peer.has_public_ipv4_address || !peer.has_public_ipv4_address)
        .cloned()
        .collect();

    let wireguard_config = WireguardConfig { interface, peers };

    println!("{wireguard_config}");
}
