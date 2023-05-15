use thiserror::Error;
use wireguard_control::InvalidKey;

#[derive(Error, Debug)]
pub enum WgMesh {
    #[error("This peer is not part of the mesh (public_key {0} not found)")]
    PeerNotPartOfMesh(String),

    #[error("Invalid interface name: {0}")]
    InvalidInterfaceName(String),

    #[error(transparent)]
    NoSuchDevice(std::io::Error),

    #[error("Public key missing")]
    NoPubkey,

    #[error("Invalid public key, could not decode: {0}, given key: {1}")]
    InvalidPublicKey(InvalidKey, String),

    #[error("Cant turn {1}:{2} into a socket address: {0}")]
    UnresolvableSocketAddress(std::io::Error, String, u16),

    #[error("Invalid IP address: {0}")]
    InvalidIpAddress(String),

    #[error("No answer from DNS server for {0}")]
    NoResolveResponse(String),

    #[error("Failed to apply wireguard config: {0}")]
    FailedToApplyConfig(std::io::Error),
}
