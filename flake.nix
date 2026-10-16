{
  description = "zapret-rust: TUI and CLI controller for zapret / zapret2 DPI bypass";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: {
        zapret-rust = pkgs.callPackage ./nix/package.nix { src = self; };
        default = self.packages.${pkgs.stdenv.hostPlatform.system}.zapret-rust;
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            pkg-config
          ];
        };
      });

      overlays.default = final: _prev: {
        zapret-rust = self.packages.${final.stdenv.hostPlatform.system}.zapret-rust;
      };

      nixosModules.default = { lib, pkgs, ... }: {
        imports = [ ./nix/module.nix ];
        services.zapret-rust.package = lib.mkDefault
          self.packages.${pkgs.stdenv.hostPlatform.system}.zapret-rust;
      };
    };
}
