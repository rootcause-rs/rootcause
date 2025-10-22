{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      {
        devShells.default = with pkgs; mkShell {
          nativeBuildInputs = [
            # nix develop shells will by default include a bash in the $PATH,
            # however this bash will be a non-interactive bash. The deviates from
            # how nix-shell works. This fix was taken from:
            #    https://discourse.nixos.org/t/interactive-bash-with-nix-develop-flake/15486
            bashInteractive

            # Rust
            # (rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
            #   extensions = [ "rust-src" "miri" ];
            #   targets = [ "x86_64-unknown-linux-gnu" ];
            # }))

            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" ];
              targets = [ "x86_64-unknown-linux-gnu" "x86_64-unknown-linux-musl" ];
            })

            cargo-deny
          ];

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

          shellHook = ''
            # nix develop shells will by default overwrite the $SHELL variable with a
            # non-interactive version of bash. The deviates from how nix-shell works.
            # This fix was taken from:
            #    https://discourse.nixos.org/t/interactive-bash-with-nix-develop-flake/15486
            #
            # See also: nixpkgs#5131 nixpkgs#6091
            export SHELL=${pkgs.bashInteractive}/bin/bash
          '';

          packages = with pkgs; [
          ];
        };
      });
}
