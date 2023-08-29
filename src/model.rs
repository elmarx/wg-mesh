#[derive(Debug, PartialEq, Clone)]
pub struct Peer {
    pub public_key: String,
    pub allowed_ips: Vec<String>,
    pub endpoint: (String, u16),
    pub site: String,
    pub has_public_ipv4_address: bool,
}
