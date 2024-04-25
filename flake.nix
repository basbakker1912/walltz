{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      perSystem = {
        config,
        self',
        pkgs,
        lib,
        system,
        ...
      }: let
        runtimeDeps = with pkgs; [postgresql];
        buildDeps = with pkgs; [pkg-config rustPlatform.bindgenHook mold clang openssl];
        devDeps = with pkgs; [rustfmt];

        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        msrv = cargoToml.package.rust-version;

        rustPackage = features:
          (pkgs.makeRustPlatform {
            cargo = pkgs.rust-bin.stable.latest.minimal;
            rustc = pkgs.rust-bin.stable.latest.minimal;
          })
          .buildRustPackage {
            inherit (cargoToml.package) name version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            buildFeatures = features;
            buildInputs = runtimeDeps;
            nativeBuildInputs = buildDeps;
            # Uncomment if your cargo tests require networking or otherwise
            # don't play nicely with the Nix build sandbox:
            # doCheck = false;
          };

        mkDevShell = rustc:
          pkgs.mkShell {
            shellHook = ''
              export RUST_SRC_PATH=${pkgs.rustPlatform.rustLibSrc}
              export RUST_LOG=info
            '';
            buildInputs = runtimeDeps;
            nativeBuildInputs = buildDeps ++ devDeps ++ [rustc];

            LD_LIBRARY_PATH = with pkgs;
              lib.makeLibraryPath [
                stdenv.cc.cc
                openssl
                # ...
              ];

            LINKER = "${pkgs.mold}";
            CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = "${pkgs.clang}/bin/clang";
            CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-C link-arg=-fuse-ld=${pkgs.mold}/bin/mold";
          };
      in {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [(import inputs.rust-overlay)];
        };

        packages.default = self'.packages.main;
        devShells.default = self'.devShells.nightly;

        packages.main = rustPackage "";

        devShells.nightly =
          mkDevShell (pkgs.rust-bin.selectLatestNightlyWith
            (toolchain: toolchain.default));
        devShells.stable = mkDevShell pkgs.rust-bin.stable.latest.default;
        devShells.msrv = mkDevShell pkgs.rust-bin.stable.${msrv}.default;
      };
    };
}
