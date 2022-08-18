{
  description = "credence";

  inputs = {
    cargo2nix.url = "github:cargo2nix/cargo2nix";
    cargo2nix.inputs.nixpkgs.follows = "nixpkgs";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.05";
  };

  outputs = { cargo2nix, nixpkgs, self }:
  let
    pname = "credence";
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ cargo2nix.overlays.default self.overlays.default ];
    };
  in {
    packages.${system}.${pname} = pkgs.${pname};
    defaultPackage.${system} = pkgs.${pname};

    overlays.default = final: prev: {
      "${pname}" = (final.rustBuilder.makePackageSet {
          rustVersion = final.rustc.version;
          packageFun = import ./Cargo.nix;
        }).workspace.${pname} {};
    };

    devShell.${system} = with pkgs; mkShell {
      buildInputs = [
        cargo
        cargo2nix.packages.${system}.cargo2nix
        openssl.dev
        pkgconfig
        rustc
        rustfmt
        zlib.dev
      ];

      shellHook = ''
        export RUST_LOG=debug
      '';
    };
  };
}
