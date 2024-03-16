{
  description = "My Rust Template";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    systems.url = "github:nix-systems/default";

    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";

    flake-utils.url = "github:numtide/flake-utils";
    flake-utils.inputs.systems.follows = "systems";

    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.tar.gz";

    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
    pre-commit-hooks.inputs.flake-compat.follows = "flake-compat";
    pre-commit-hooks.inputs.flake-utils.follows = "flake-utils";
    pre-commit-hooks.inputs.nixpkgs.follows = "nixpkgs";
    pre-commit-hooks.inputs.nixpkgs-stable.follows = "nixpkgs";


    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.flake = false;
  };

  outputs = inputs@{ systems, flake-parts, rust-overlay, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        # To import a flake module
        # 1. Add foo to inputs
        # 2. Add foo as a parameter to the outputs function
        # 3. Add here: foo.flakeModule

        ./dev/flake-module.nix
      ];
      systems = import systems;
      perSystem = { pkgs, system, ... }:
        let
          nativeBuildInputs = with pkgs; [
            # Build-time Additional Dependencies
          ];
          buildInputs = with pkgs; [
            # Run-time Additional Dependencies
          ];

          rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./toolchain.toml;

          rustPlatform = pkgs.makeRustPlatform {
            cargo = rust-toolchain;
            rustc = rust-toolchain;
          };
        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              (import rust-overlay)
            ];
          };

          # Per-system attributes can be defined here. The self' and inputs'
          # module parameters provide easy access to attributes of the same
          # system.
          packages.default =
            let
              # Same as in Cargo.toml
              inherit ((builtins.fromTOML (builtins.readFile ./Cargo.toml)).package) name;
            in
            rustPlatform.buildRustPackage {
              inherit name buildInputs nativeBuildInputs;
              src = ./.;
              cargoLock = {
                lockFile = ./Cargo.lock;
              };
            };
        };
      flake = {
        # The usual flake attributes can be defined here, including system-
        # agnostic ones like nixosModule and system-enumerating ones, although
        # those are more easily expressed in perSystem.

      };
    };
}
