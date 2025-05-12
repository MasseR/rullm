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

  cargoHash = "sha256-4a9ElX76trKA10UxuPWN4d1yoQvrSkC7O1bGA7uS608=";

})
