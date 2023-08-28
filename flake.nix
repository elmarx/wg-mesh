{
  description = "A devShell example";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml));
        supportedSystems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
        forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f system);
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            (rust-bin.stable.latest.default.override { extensions = [ "rust-src" ]; })
            cargo-outdated
            cargo-watch

            terraform
            nixpkgs-fmt
          ] ++ lib.optionals (stdenv.isDarwin) [ darwin.apple_sdk.frameworks.Security ];

          nativeBuildInputs = [ pkgs.libclang ];
        };

        defaultPackage = pkgs.callPackage ./default.nix { };
      }
    );
}
