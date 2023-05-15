use std::fmt::{Display, Formatter};

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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut config = self.interface.to_string();

        for peer in &self.peers {
            config.push_str(&peer.to_string());
        }

        write!(f, "{config}")
    }
}

impl Display for Interface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"
            [Interface]
            ListenPort = {},
            PrivateKey = {}
            "#,
            self.listen_port, self.private_key
        )
    }
}

impl Display for Peer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"
            [Peer]
            Endpoint = {}:{},
            PublicKey = {}
            AllowedIPs = {}
            PersistentKeepalive = {}
            "#,
            self.endpoint.0,
            self.endpoint.1,
            self.public_key,
            self.allowed_ips.join(", "),
            self.persistent_keepalive,
        )
    }
}
