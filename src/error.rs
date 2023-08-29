use rsdns::Error as RsdnsError;
use thiserror::Error;
use wireguard_control::InvalidKey;

#[derive(Error, Debug)]
pub enum WgMesh {
    #[error(transparent)]
    Mesh(#[from] Mesh),

    #[error(transparent)]
    NodeRepository(#[from] NodeRepository),

    #[error(transparent)]
    Routing(#[from] Routing),

    #[error(transparent)]
    Wireguard(#[from] Wireguard),
}

#[derive(Error, Debug)]
pub enum Mesh {
    #[error("This peer is not part of the mesh (public_key {0} not found)")]
    PeerNotPartOfMesh(String),
}

#[derive(Error, Debug)]
pub enum NodeRepository {
    #[error(transparent)]
    Rsdns(#[from] RsdnsError),

    #[error("Cant turn {1} into a socket address: {0}")]
    UnresolvableSocketAddress(std::io::Error, String),

    #[error("Can't resolve nameserver: {0}")]
    InvalidNameserver(String),

    #[error("Missing TXT-Record for {0} (expecting record to hold pubkey)")]
    MissingPubkeyRecord(String),
}

#[derive(Error, Debug)]
pub enum Routing {
    #[error(transparent)]
    NetlinkError(#[from] rtnetlink::Error),

    #[error("Netlink interface {0} not found")]
    NoSuchInterface(String),
}

#[derive(Error, Debug)]
pub enum Wireguard {
    #[error("Invalid interface name: {0}")]
    InvalidInterfaceName(String),

    #[error("No answer from DNS server for {0}")]
    NoResolveResponse(String),

    #[error("Cant turn {1}:{2} into a socket address: {0}")]
    UnresolvableSocketAddress(std::io::Error, String, u16),

    #[error("Invalid IP address: {0}")]
    InvalidIpAddress(String),

    #[error("Public key missing")]
    NoPubkey,

    #[error(transparent)]
    NoSuchDevice(std::io::Error),

    #[error("Failed to apply wireguard config: {0}")]
    FailedToApplyConfig(std::io::Error),

    #[error("Invalid public key, could not decode: {0}, given key: {1}")]
    InvalidPublicKey(InvalidKey, String),
}
