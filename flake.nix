{
  description = "A very basic flake";
  inputs = {
  };

  outputs = { self, nixpkgs }: {
    packages.x86_64-linux = with nixpkgs.legacyPackages.x86_64-linux; rec {
      mcp-mealie-server = callPackage ./mcp-mealie-server.nix {};
      rullm = callPackage ./rullm.nix {};
    };

    devShell.x86_64-linux =
      with nixpkgs.legacyPackages.x86_64-linux;
      mkShell {
        buildInputs = [
          pkg-config
          openssl
          exercism
          cargo
          rustc
          evcxr
          rustup
          nodePackages.npm
          rustfmt
        ];

        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      };

  };
}
