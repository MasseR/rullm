{
  lib,
  rustPlatform,
  openssl,
  pkg-config
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "mcp-mealie-server";
  version = "0.0.1";

  src = ./mcp-mealie-server;

  nativeBuildInputs = [openssl.dev pkg-config];
  PKG_CONFIG_PATH = "${openssl.dev}/lib/pkgconfig";

  cargoHash = "sha256-L8w1u1L+YTwlS0Yd3paCxVKcoi3qr4wLEoGORqXY12c=";

})
