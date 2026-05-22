use clap::Parser;

#[derive(Parser)]
#[command(about = "Keep a WireGuard mesh in sync via DNS")]
pub struct Cli {
    /// DNS record containing the list of mesh peers
    pub mesh_record: String,

    /// WireGuard interface name
    #[arg(short, long, default_value = "wg0")]
    pub interface: String,

    /// DNS resolver address (e.g. 1.1.1.1:53). Defaults to the system resolver.
    #[arg(short, long, env = "WG_MESH_RESOLVER")]
    pub resolver: Option<String>,
}
