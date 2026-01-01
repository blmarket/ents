{
  description = "A minimal entity framework";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      crane,
      ...
    }:
    flake-utils.lib.eachSystem
      [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ]
      (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ ];
          };
          pkgsCross = import nixpkgs {
            inherit system;
            crossSystem = {
              config = "aarch64-unknown-linux-gnu";
              rust.rustcTarget = "aarch64-unknown-linux-gnu";
            };
          };
          craneLib = crane.mkLib pkgs;
        in
        {
          devShells.default = pkgs.mkShell {
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.nixfmt-rfc-style
              pkgs.cargo
              pkgs.clippy # for cargo clippy
              pkgs.rustfmt
              pkgs.rustc
              pkgs.deno
            ];

            buildInputs = [
              pkgs.openssl
              pkgs.sqlite
            ];
          };

          formatter = pkgs.writeScriptBin "format" ''
            #!${pkgs.bash}/bin/bash
            ${pkgs.nixfmt-rfc-style}/bin/nixfmt flake.nix
          '';
        }
      );
}
