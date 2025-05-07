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

  cargoHash = "sha256-0AHUGTGPQcwO39XwGiz546k6tEKZWZsIH9/aJhbesUs=";

})
