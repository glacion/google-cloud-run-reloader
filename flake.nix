{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustToolchain = pkgs.rust-bin.selectLatestNightlyWith (
          toolchain:
          toolchain.default.override {
            extensions = [
              "rust-analyzer"
              "rust-src"
              "rustfmt"
              "clippy"
            ];
          }
        );
      in
      {
        packages = rec {
          default = release;
          release = pkgs.rustPlatform.buildRustPackage {
            name = "reloader";
            version = self.shortRev;
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
          };

          docker = pkgs.dockerTools.buildImage {
            name = "reloader";
            tag = self.shortRev;
            copyToRoot = pkgs.buildEnv {
              name = "root";
              paths = [
                release
                pkgs.dockerTools.caCertificates
                pkgs.dockerTools.fakeNss
              ];
            };
            config = {
              Env = [ "RUST_LOG=info" ];
              Entrypoint = [ "/bin/reloader" ];
            };
          };

          dockerStream = pkgs.dockerTools.streamLayeredImage {
            name = "reloader";
            tag = self.shortRev;
            contents = [
              release
              pkgs.dockerTools.caCertificates
              pkgs.dockerTools.fakeNss
            ];
            config = {
              Env = [ "RUST_LOG=info" ];
              Entrypoint = [ "/bin/reloader" ];
            };
          };
        };

        devShells = {
          default = pkgs.mkShell {
            packages = [
              rustToolchain
              pkgs.direnv
            ];
          };
        };
      }
    );
}
