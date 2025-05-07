{
  description = "A very basic flake";
  inputs = {
  };

  outputs = { self, nixpkgs }: {

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
        ];
        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      };

  };
}
