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
        name = "reloader";
        version = self.rev or self.dirtyRev;

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
          default = pkgs.rustPlatform.buildRustPackage {
            inherit name version;
            cargoLock.lockFile = ./Cargo.lock;
            src = ./.;
          };
          stream = pkgs.dockerTools.streamLayeredImage {
            inherit name;
            tag = self.dirtyShortRev;
            contents = [
              default
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
              pkgs.skopeo
            ];
          };
        };
      }
    );
}
