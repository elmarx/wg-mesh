use std::net::ToSocketAddrs;
use std::str::FromStr;

use wireguard_control::{AllowedIp, Key, PeerConfigBuilder};

use crate::error::WgMesh as WgMeshError;
use crate::error::WgMesh::{InvalidIpAddress, InvalidPublicKey};
use crate::model::Peer;

impl TryInto<PeerConfigBuilder> for &Peer {
    type Error = WgMeshError;

    fn try_into(self) -> Result<PeerConfigBuilder, Self::Error> {
        let allowed_ips = self
            .allowed_ips
            .iter()
            .map(|a| AllowedIp::from_str(a).map_err(|_| InvalidIpAddress(a.clone())))
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
            .map_err(|e| {
                WgMeshError::UnresolvableSocketAddress(e, self.endpoint.0.clone(), self.endpoint.1)
            })
            .and_then(|mut i| {
                i.next()
                    .ok_or(WgMeshError::NoResolveResponse(self.endpoint.0.clone()))
            })
            .ok();

        if let Some(endpoint) = endpoint {
            Ok(o.set_endpoint(endpoint))
        } else {
            Ok(o)
        }
    }
}
