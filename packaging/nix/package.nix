{ lib
, rustPlatform
, makeWrapper
, src
, nftables
, iptables
, curl
, libcap
, procps
, polkit
, coreutils
}:

let
  runtimeDeps = [
    nftables
    iptables
    curl
    libcap
    procps
    polkit
    coreutils
  ];
in
rustPlatform.buildRustPackage {
  pname = "zapret-rust";
  version = "1.0.0";

  inherit src;

  cargoLock.lockFile = "${src}/Cargo.lock";

  nativeBuildInputs = [ makeWrapper ];

  doCheck = false;

  postInstall = ''
    wrapProgram $out/bin/zapret-rust \
      --prefix PATH : ${lib.makeBinPath runtimeDeps}:/run/wrappers/bin \
      --set-default ZAPRET_CACHE_DIR /var/lib/zapret-rust
  '';

  meta = {
    description = "TUI and CLI controller for zapret and zapret2 DPI bypass";
    homepage = "https://github.com/soufied/zapret-tui-rs";
    license = lib.licenses.gpl3Only;
    mainProgram = "zapret-rust";
    platforms = lib.platforms.linux;
  };
}
