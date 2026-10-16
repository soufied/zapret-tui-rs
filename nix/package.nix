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
  # Tools zapret-rust runs as external commands.
  runtimeDeps = [
    nftables   # nft: default firewall backend
    iptables   # iptables: alternative backend
    curl       # connectivity probes and downloads
    libcap     # setcap
    procps     # pgrep / pkill
    polkit     # pkexec for unprivileged launches
    coreutils  # id and friends
  ];
in
rustPlatform.buildRustPackage {
  pname = "zapret-rust";
  version = "2.3.0";

  inherit src;

  # Requires Cargo.lock to be committed to the repository (see README).
  cargoLock.lockFile = "${src}/Cargo.lock";

  nativeBuildInputs = [ makeWrapper ];

  # Tests may need root or network access, so they are not run in the sandbox.
  doCheck = false;

  postInstall = ''
    wrapProgram $out/bin/zapret-rust \
      --prefix PATH : ${lib.makeBinPath runtimeDeps}:/run/wrappers/bin \
      --set-default ZAPRET_CACHE_DIR /var/lib/zapret-rust
  '';

  meta = {
    description = "TUI and CLI controller for zapret and zapret2 DPI bypass";
    homepage = "https://github.com/soufied/zapret-tui-rs";
    license = lib.licenses.mit;
    mainProgram = "zapret-rust";
    platforms = lib.platforms.linux;
  };
}
