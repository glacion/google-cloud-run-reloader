{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix/release-0.12";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      cargo2nix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        name = "reloader";
        version = self.rev or self.dirtyRev;

        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            rust-overlay.overlays.default
            cargo2nix.overlays.default
          ];
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

        rustPkgs = pkgs.rustBuilder.makePackageSet {
          rustVersion = pkgs.rust-bin.stable.latest.default.version;
          packageFun = import ./Cargo.nix;
          packageOverrides = pkgs: [
            pkgs.rustBuilder.overrides.capLints
          ];
        };

        package = rustPkgs.workspace.reloader { };
      in
      {
        packages = rec {
          default = package;
          stream = pkgs.dockerTools.streamLayeredImage {
            inherit name;
            tag = version;
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

        checks.clippy = pkgs.rustPlatform.buildRustPackage {
          inherit version;
          pname = "clippy";
          doCheck = true;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [
            rustToolchain
          ];
          checkPhase = ''
            runHook preCheck
            cargo clippy --workspace --all-targets --offline --locked -- -D warnings
            runHook postCheck
          '';
        };

        devShells.default = rustPkgs.workspaceShell {
          nativeBuildInputs = [
            rustToolchain
            pkgs.cargo
            pkgs.direnv
            pkgs.skopeo
          ];
        };
      }
    );
}
