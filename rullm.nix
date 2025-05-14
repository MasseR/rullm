{
  lib,
  rustPlatform,
  openssl,
  pkg-config
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "rullm";
  version = "0.0.1";

  src = ./rullm;

  nativeBuildInputs = [openssl.dev pkg-config];
  PKG_CONFIG_PATH = "${openssl.dev}/lib/pkgconfig";

  cargoHash = "sha256-bCLe38aKaG3gK30ZC5OC4KMCyNN+kYqRPz0p5K1126U=";

})
