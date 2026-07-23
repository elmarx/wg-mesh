# Wireguard Mesh

wg-mesh is a tool to create a peer-to-peer wireguard mesh with configuration stored in DNS.

## use case

My goal is to connect different peers with public IP addresses and private/NATed/dynamic IP addresses, so they can reach each other directly (just how IP/IPv6 was invented). 

I want to run k3s on top.

Adding new peers to the mesh should not require a provisioning-step to introduce the node to all other peers.

Wireguard should only be used to provide directly accessible, static IP-addresses, where possible (e.g. nodes in the same subnet) a direct connection should be used.

## Setup for manual local testing

```shell
ip link add dev wg0 type wireguard
ip addr add 10.0.0.1 dev wg0
ip link set up dev wg0
echo "yFlXT2A5TTg9xcPfxolP0arOR+Ua+z8NlAcaKoTpm2E=" > /tmp/wg-mesh-privatekey
wg set wg0 listen-port 51820 private-key /tmp/wg-mesh-privatekey
```

## execution

```shell
wg-mesh _p2p.example.com
```

## DNS setup

The initial "mesh"-records points to the list of mesh-peers (via TXT record to hostnames).

Each mesh-peer comes with an additional `_wireguard`-prefix entry, that has a TXT record for the
pubkey and A/AAAA records pointing to the list of allowed ips.

### Zone file for example.com

```
_p2p                                IN      TXT     "node-a.example.com"
                                    IN      TXT     "node-b.example.com"
                                    IN      TXT     "node-c.example.net"
                        
_wireguard.node-a.example.com.      IN      TXT     "MEsH/tKCQF/ZmJ8QUWo5M3fsInoLM604z2XvYi4TVG8="
_wireguard.node-a.example.com.      IN      A       10.0.0.1
                                    IN      A       192.168.1.1

_wireguard.node-b                   IN      TXT     "meshXJx7qntmyyvZhm8awrXXXFtbjD+OZzYhWli9jR0="
_wireguard.node-b                   IN      A       10.0.0.2
                                    IN      A       192.168.1.2

```

### Zone file for example.net

```
_wireguard.node-c                   IN      TXT     "mEshKpyb8WxRc8OyF9sH5bQVaSBFa1Zk0Red6x704WY="
                                    IN      A       10.0.0.3
                                    IN      A       192.168.1.3        
```

### Nixos module

You may use this as a nixos module (via flake-parts):

```nix
# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    wg-mesh.url = "github:elmarx/wg-mesh";
    wg-mesh.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, wg-mesh, ... }: {
    nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        wg-mesh.nixosModules.default
        ./configuration.nix
      ];
    };
  };
}
```

`wg-mesh.nixosModules.default` only manages the `wg-mesh` systemd service itself
(`wg-mesh.enable`, `wg-mesh.package`, `wg-mesh.domain` and `wg-mesh.interface`); setting up the
wireguard interface (netdev, network, firewall, private key) is deployment-specific and left to
you, e.g. via `systemd-networkd`:

```nix
# configuration.nix
{ config, ... }:

{
  # the private key is expected to be provided as a credential, e.g. via
  # LoadCredential, agenix, sops-nix, ...
  systemd.services.systemd-networkd.serviceConfig.LoadCredential = [
    "wireguard-wg0-private-key:/run/secrets/wg-mesh-private-key"
  ];

  systemd.network = {
    wait-online.ignoredInterfaces = [ "wg0" ];
    netdevs."50-wg0" = {
      netdevConfig = {
        Kind = "wireguard";
        Name = "wg0";
      };
      wireguardConfig = {
        PrivateKey = "@wireguard-wg0-private-key";
        ListenPort = 51820;
        RouteTable = "main";
      };
    };
    networks.wg0 = {
      matchConfig.Name = "wg0";
      address = [ "10.0.0.1/32" ];
    };
  };

  networking.firewall.allowedUDPPorts = [ 51820 ];

  wg-mesh = {
    enable = true;
    domain = "_p2p.example.com";
    # interface = "wg0"; # default, only needed if you named the netdev differently
  };
}
```

