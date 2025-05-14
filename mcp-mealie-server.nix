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

  cargoHash = "sha256-HkmqZWM1sp1z9gwWdxmrC3dcWnswGRvL0BsQY0b4JD4=";

})
