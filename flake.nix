{
  description = "winetop — htop for Wine prefixes";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "winetop";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          meta = with pkgs.lib; {
            description = "htop for Wine prefixes";
            homepage = "https://github.com/akovari/winetop";
            license = licenses.mit;
            mainProgram = "winetop";
          };
        };
        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/winetop";
        };
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [ rustc cargo clippy rustfmt ];
        };
      });
}
