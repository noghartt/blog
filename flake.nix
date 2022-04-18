{
  description = "A flake to my blog project";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      {
        devShell = pkgs.mkShell {
          packages = [ pkgs.nodePackages.pnpm ];

          buildInputs = with pkgs; [
            nodejs
          ];

          shellHook = ''
            pnpm i
          '';
        };
      }
    );
}
