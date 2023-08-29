use crate::error;
use crate::error::NodeRepository::{InvalidNameserver, UnresolvableSocketAddress};
use resolv_conf::ScopedIp;
use std::fs::File;
use std::io::Read;
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs};

pub fn from_resolv_conf() -> SocketAddr {
    let mut buf = Vec::with_capacity(4096);
    let mut f = File::open("/etc/resolv.conf").expect("open resolv.conf");
    f.read_to_end(&mut buf).expect("read resolv.conf");

    // Parse the buffer
    let cfg = resolv_conf::Config::parse(&buf).expect("invalid resolv.conf");

    let nameserver = cfg
        .nameservers
        .first()
        .expect("no nameserver in resolv.conf");

    match nameserver {
        ScopedIp::V4(a) => SocketAddr::V4(SocketAddrV4::new(*a, 53)),
        ScopedIp::V6(a, None) => SocketAddr::V6(SocketAddrV6::new(*a, 53, 0, 0)),
        ScopedIp::V6(_, Some(_)) => todo!(),
    }
}

pub fn from_address(address: &str) -> Result<SocketAddr, error::NodeRepository> {
    address
        .to_socket_addrs()
        .map_err(|e| UnresolvableSocketAddress(e, address.to_string()))?
        .next()
        .ok_or_else(|| InvalidNameserver(address.to_string()))
}
