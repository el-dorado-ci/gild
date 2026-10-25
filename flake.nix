{
  description = "gild dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, git-hooks }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        pre-commit-check = git-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            rustfmt.enable = true;
            clippy.enable = true;
          };
        };
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;

        gild = pkgs.rustPlatform.buildRustPackage {
          pname = manifest.name;
          inherit (manifest) version;

          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          meta = {
            inherit (manifest) description;
            homepage = manifest.repository;
            license = pkgs.lib.licenses.mit;
            mainProgram = "gild";
          };
        };
      in
      {
        checks.pre-commit = pre-commit-check;

        packages = {
          default = gild;
          gild-cli = gild;
        };

        apps.default = {
          type = "app";
          program = "${gild}/bin/gild";
        };

        devShells.default = pkgs.mkShell {
          # Installs the hooks into .git/hooks and writes .pre-commit-config.yaml
          inherit (pre-commit-check) shellHook;
          buildInputs = pre-commit-check.enabledPackages;

          packages = [
            pkgs.rustc
            pkgs.cargo
            pkgs.rust-analyzer
          ];

          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });
}