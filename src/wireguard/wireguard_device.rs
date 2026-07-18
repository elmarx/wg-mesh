use std::str::FromStr;

use crate::error;
use wireguard_control::{Backend, Device, DeviceUpdate, InterfaceName};

use crate::error::Wireguard::{FailedToApplyConfig, InvalidInterfaceName, NoPubkey, NoSuchDevice};
use crate::model::Peer;
use crate::traits::Wireguard;

#[cfg(target_os = "linux")]
pub const BACKEND: Backend = Backend::Kernel;

#[derive(Debug)]
pub struct WireguardImpl {
    interface_name: InterfaceName,
}

impl WireguardImpl {
    pub fn new(interface_name: &str) -> Result<WireguardImpl, error::Wireguard> {
        let interface_name = InterfaceName::from_str(interface_name)
            .map_err(|_| InvalidInterfaceName(interface_name.to_string()))?;

        Ok(WireguardImpl { interface_name })
    }
}

impl Wireguard for WireguardImpl {
    fn replace_peers(&self, peers: &[Peer]) -> Result<(), error::Wireguard> {
        let peer_config_builders = peers
            .iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;

        let update = DeviceUpdate::new()
            .replace_peers()
            .add_peers(&peer_config_builders);

        update
            .apply(&self.interface_name, BACKEND)
            .map_err(FailedToApplyConfig)
    }
    fn get_interface_pubkey(&self) -> Result<String, error::Wireguard> {
        let device = Device::get(&self.interface_name, BACKEND).map_err(NoSuchDevice)?;
        let interface_pubkey = device.public_key.ok_or(NoPubkey)?;
        Ok(interface_pubkey.to_base64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    #[test]
    fn valid_interface_name_succeeds() {
        assert!(WireguardImpl::new("wg0").is_ok());
    }

    #[test]
    fn empty_interface_name_returns_error() {
        assert_matches!(
            WireguardImpl::new(""),
            Err(error::Wireguard::InvalidInterfaceName(_))
        );
    }

    #[test]
    fn too_long_interface_name_returns_error() {
        // Linux interface names are limited to IFNAMSIZ-1 = 15 characters
        let long_name = "a".repeat(16);
        assert_matches!(
            WireguardImpl::new(&long_name),
            Err(error::Wireguard::InvalidInterfaceName(_))
        );
    }
}
