{ inputs, ... }: {
  imports = [ inputs.pre-commit-hooks.flakeModule ];

  perSystem = { self', config, pkgs, ... }:
    let
      treefmtRuntimeInputs = with pkgs; [
        treefmt
        rustfmt
      ];

      treefmtWrapper = pkgs.writeShellApplication {
        name = "treefmt";
        runtimeInputs = treefmtRuntimeInputs;
        text =
          ''
            exec treefmt "$@"
          '';
      };
    in
    {
      devShells.default = pkgs.mkShell {
        shellHook = ''
          ${config.pre-commit.installationScript}
        '';

        packages = treefmtRuntimeInputs ++ (with pkgs; [
          bacon
          cargo-expand
          cargo-nextest
          cargo-show-asm
          cargo-watch

          # Debugging
          # lldb
        ]);

        inputsFrom = builtins.attrValues (self'.packages or [ ]);
      };

      formatter = treefmtWrapper;

      pre-commit = {
        check.enable = true;

        settings = {
          excludes = [ "flake.lock" ];
          settings.treefmt.package = treefmtWrapper;

          hooks = {
            treefmt.enable = true;
          };
        };
      };
    };
}
