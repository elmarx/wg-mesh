use std::fmt::Display;

pub struct Interface {
    pub listen_port: u16,
    pub private_key: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Peer {
    pub public_key: String,
    pub allowed_ips: Vec<String>,
    pub endpoint: (String, u16),
    pub site: String,
    pub persistent_keepalive: u16,
    pub has_public_ipv4_address: bool,
}

pub struct WireguardConfig {
    pub interface: Interface,
    pub peers: Vec<Peer>,
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
