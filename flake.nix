{
  description = "Blog Flakefile";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    # Recent stable Rust for engine/, independent of the pinned nixpkgs.
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        rust = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };
      in
      with pkgs;
      {
        devShells.default = mkShell {
          packages = [
            nodejs_23
            nodePackages.yarn
            pnpm
            yarn

            # engine/ (Rust static site generator)
            rust
          ];
        };
      }
    );
}
