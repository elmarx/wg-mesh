use crate::error::WgMesh as WgMeshError;
use crate::model::Peer;

/// given a list of all nodes of the mesh, filter out all nodes we need to actually peer with
pub fn filter_peers(interface_pubkey: &str, mut peers: Vec<Peer>) -> Vec<Peer> {
    let this_peer = peers
        .iter()
        .position(|peer| peer.public_key == interface_pubkey)
        .ok_or(WgMeshError::PeerNotPartOfMesh(interface_pubkey.to_string()))
        .unwrap();

    let this_peer = peers.swap_remove(this_peer);

    let peers: Vec<_> = peers
        .into_iter()
        .filter(|peer| *peer != this_peer)
        .filter(|peer| peer.site != this_peer.site)
        .filter(|peer| !this_peer.has_public_ipv4_address || !peer.has_public_ipv4_address)
        .collect();

    peers
}
