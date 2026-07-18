use std::net::ToSocketAddrs;
use std::str::FromStr;

use crate::error;
use error::Wireguard::{NoResolveResponse, UnresolvableSocketAddress};
use wireguard_control::{AllowedIp, Key, PeerConfigBuilder};

use crate::error::Wireguard::{InvalidIpAddress, InvalidPublicKey};
use crate::model::Peer;

impl TryInto<PeerConfigBuilder> for &Peer {
    type Error = error::Wireguard;

    fn try_into(self) -> Result<PeerConfigBuilder, Self::Error> {
        let allowed_ips = self
            .allowed_ips
            .iter()
            .map(|a| AllowedIp::from_str(a).map_err(|()| InvalidIpAddress(a.clone())))
            .collect::<Result<Vec<_>, _>>()?;

        let public_key = Key::from_base64(&self.public_key)
            .map_err(|e| InvalidPublicKey(e, self.public_key.clone()))?;

        let o = PeerConfigBuilder::new(&public_key)
            .add_allowed_ips(&allowed_ips)
            .set_persistent_keepalive_interval(25);

        // the endpoint is optional
        let endpoint = self
            .endpoint
            .to_socket_addrs()
            .map_err(|e| UnresolvableSocketAddress(e, self.endpoint.0.clone(), self.endpoint.1))
            .and_then(|mut i| i.next().ok_or(NoResolveResponse(self.endpoint.0.clone())))
            .ok();

        if let Some(endpoint) = endpoint {
            Ok(o.set_endpoint(endpoint))
        } else {
            Ok(o)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Peer;
    use std::assert_matches;
    use std::time::Duration;

    // A valid 32-byte WireGuard public key encoded as base64 (44 chars)
    const VALID_PUBKEY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

    fn make_peer(pubkey: &str, allowed_ips: Vec<&str>, endpoint: (&str, u16)) -> Peer {
        Peer {
            public_key: pubkey.to_string(),
            allowed_ips: allowed_ips.into_iter().map(str::to_string).collect(),
            endpoint: (endpoint.0.to_string(), endpoint.1),
            site: ".example.com".to_string(),
            has_public_ipv4_address: false,
            ttl: Duration::from_mins(5),
        }
    }

    #[test]
    fn valid_peer_converts_successfully() {
        let peer = make_peer(VALID_PUBKEY, vec!["10.0.0.1/32"], ("127.0.0.1", 51820));
        let result: Result<PeerConfigBuilder, _> = (&peer).try_into();
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_pubkey_returns_error() {
        let peer = make_peer("not-a-valid-key", vec!["10.0.0.1/32"], ("127.0.0.1", 51820));
        let result: Result<PeerConfigBuilder, _> = (&peer).try_into();
        assert_matches!(result, Err(error::Wireguard::InvalidPublicKey(_, _)));
    }

    #[test]
    fn invalid_allowed_ip_returns_error() {
        let peer = make_peer(VALID_PUBKEY, vec!["not-an-ip"], ("127.0.0.1", 51820));
        let result: Result<PeerConfigBuilder, _> = (&peer).try_into();
        assert_matches!(result, Err(error::Wireguard::InvalidIpAddress(_)));
    }

    #[test]
    fn empty_allowed_ips_converts_successfully() {
        let peer = make_peer(VALID_PUBKEY, vec![], ("127.0.0.1", 51820));
        let result: Result<PeerConfigBuilder, _> = (&peer).try_into();
        assert!(result.is_ok());
    }

    #[test]
    fn unresolvable_hostname_endpoint_is_silently_omitted() {
        // Unresolvable endpoints are treated as optional and silently skipped
        let peer = make_peer(
            VALID_PUBKEY,
            vec![],
            ("this.host.does.not.exist.invalid", 51820),
        );
        let result: Result<PeerConfigBuilder, _> = (&peer).try_into();
        assert!(result.is_ok());
    }
}
