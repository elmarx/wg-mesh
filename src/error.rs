use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum WgMesh {
    #[error("This peer is not part of the mesh (public_key {0} not found)")]
    PeerNotPartOfMesh(String),

    #[error("Invalid interface name: {0}")]
    InvalidInterfaceName(String),

    #[error(transparent)]
    NoSuchDevice(std::io::Error),

    #[error("Public key missing")]
    NoPubkey,
}
