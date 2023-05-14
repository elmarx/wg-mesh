use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum WgMesh {
    #[error("This peer is not part of the mesh (public_key {0} not found)")]
    PeerNotPartOfMesh(String),

    #[error("The pubkey is missing")]
    PubkeyMissing,

    #[error("The private key is missing")]
    PrivateKeyMissing,
}
