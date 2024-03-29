use crate::error;
use async_trait::async_trait;
use error::Routing::NoSuchInterface;
use futures::TryStreamExt;
use ipnet::Ipv4Net;
use netlink_packet_core::ErrorMessage;
use nix::errno::Errno;
use rtnetlink::Error::NetlinkError;
use rtnetlink::RouteHandle;

use crate::model::Peer;
use crate::traits::RoutingService;

pub struct RoutingServiceImpl {
    route_handle: RouteHandle,
    handle: rtnetlink::Handle,
    interface_name: String,
}

impl RoutingServiceImpl {
    pub fn new(handle: rtnetlink::Handle, interface_name: &str) -> RoutingServiceImpl {
        let route_handle = handle.route();

        RoutingServiceImpl {
            route_handle,
            handle,
            interface_name: interface_name.to_string(),
        }
    }
}

#[async_trait(?Send)]
impl RoutingService for RoutingServiceImpl {
    async fn add_routes(&self, peers: &[Peer]) -> Result<(), error::Routing> {
        let mut link_handle = self.handle.link();

        let interface = link_handle
            .get()
            .match_name(self.interface_name.clone())
            .execute()
            .try_next()
            .await?;

        let interface =
            interface.ok_or_else(|| NoSuchInterface(self.interface_name.to_string()))?;

        let route_add_requests: Vec<_> = peers
            .iter()
            .flat_map(|peer| {
                peer.allowed_ips
                    .iter()
                    .map(|i| {
                        i.parse::<Ipv4Net>()
                            .expect("got invalid IPv4 address for Peer's allowed_ips")
                    })
                    .map(|ip| {
                        self.route_handle
                            .add()
                            .v4()
                            .input_interface(interface.header.index)
                            .output_interface(interface.header.index)
                            .destination_prefix(ip.addr(), ip.prefix_len())
                    })
            })
            .collect();

        for r in route_add_requests {
            r.execute()
                .await
                .or_else(|e| -> Result<(), error::Routing> {
                    match e {
                        // TODO: this is not very elegant, so better check in the first place if something has to be added or not
                        NetlinkError(ErrorMessage {
                            code: Some(code), ..
                        }) if i32::from(-code) == Errno::EEXIST as i32 => Ok(()),
                        err => Err(error::Routing::NetlinkError(err)),
                    }
                })?;
        }

        Ok(())
    }
}
