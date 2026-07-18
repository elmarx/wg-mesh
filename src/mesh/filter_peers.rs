use crate::error;
use crate::model::Peer;

/// given a list of all nodes of the mesh, filter out all nodes we need to actually peer with
pub fn filter_peers(
    interface_pubkey: &str,
    mut peers: Vec<Peer>,
) -> Result<Vec<Peer>, error::Mesh> {
    let this_peer = peers
        .iter()
        .position(|peer| peer.public_key == interface_pubkey)
        .ok_or(error::Mesh::PeerNotPartOfMesh(interface_pubkey.to_string()))?;

    let this_peer = peers.swap_remove(this_peer);

    let peers: Vec<_> = peers
        .into_iter()
        .filter(|peer| *peer != this_peer)
        .filter(|peer| peer.site != this_peer.site)
        .filter(|peer| !this_peer.has_public_ipv4_address || !peer.has_public_ipv4_address)
        .collect();

    Ok(peers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;
    use std::time::Duration;

    fn make_peer(pubkey: &str, site: &str, has_public_ipv4: bool) -> Peer {
        Peer {
            public_key: pubkey.to_string(),
            allowed_ips: vec![],
            endpoint: ("example.com".to_string(), 51820),
            site: site.to_string(),
            has_public_ipv4_address: has_public_ipv4,
            ttl: Duration::from_mins(5),
        }
    }

    #[test]
    fn self_is_excluded_from_result() {
        let peers = vec![
            make_peer("key-self", ".site-a", false),
            make_peer("key-other", ".site-b", false),
        ];
        let result = filter_peers("key-self", peers).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].public_key, "key-other");
    }

    #[test]
    fn peer_not_in_mesh_returns_error() {
        let peers = vec![make_peer("key-other", ".site-b", false)];
        assert_matches!(
            filter_peers("key-self", peers),
            Err(error::Mesh::PeerNotPartOfMesh(k)) if k == "key-self"
        );
    }

    #[test]
    fn peers_on_same_site_are_excluded() {
        let peers = vec![
            make_peer("key-self", ".site-a", false),
            make_peer("key-same-site", ".site-a", false),
            make_peer("key-other-site", ".site-b", false),
        ];
        let result = filter_peers("key-self", peers).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].public_key, "key-other-site");
    }

    #[test]
    fn peer_with_public_ip_excluded_when_self_also_public() {
        let peers = vec![
            make_peer("key-self", ".site-a", true),
            make_peer("key-other-public", ".site-b", true),
            make_peer("key-other-private", ".site-c", false),
        ];
        let result = filter_peers("key-self", peers).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].public_key, "key-other-private");
    }

    #[test]
    fn all_peers_from_other_sites_kept_when_self_is_private() {
        let peers = vec![
            make_peer("key-self", ".site-a", false),
            make_peer("key-public", ".site-b", true),
            make_peer("key-private", ".site-c", false),
        ];
        let result = filter_peers("key-self", peers).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn empty_result_after_all_filtered() {
        let peers = vec![
            make_peer("key-self", ".site-a", false),
            make_peer("key-same-site", ".site-a", false),
        ];
        let result = filter_peers("key-self", peers).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn only_self_in_mesh_returns_empty() {
        let peers = vec![make_peer("key-self", ".site-a", false)];
        let result = filter_peers("key-self", peers).unwrap();
        assert!(result.is_empty());
    }
}
