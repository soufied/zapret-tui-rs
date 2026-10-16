# zapret-tui-rs

A terminal UI (TUI) and command-line controller for [**zapret**](https://github.com/bol-van/zapret) (`nfqws` / `winws`) and [**zapret2**](https://github.com/bol-van/zapret2) (`nfqws2` / `winws2`) by [bol-van](https://github.com/bol-van).

It automates DPI (deep packet inspection) bypass setups for commonly blocked services such as Discord and YouTube. It downloads engine binaries and strategy bundles, sets up firewall rules, and can run as a background service on Linux and Windows.

---

## Contents

- [Features](#features)
- [Platform support](#platform-support)
- [Quick start](#quick-start)
- [Installation](#installation)
  - [Requirements](#requirements)
  - [Build from source (any Linux)](#build-from-source-any-linux)
  - [Debian and Ubuntu](#debian-and-ubuntu)
  - [Fedora](#fedora)
  - [Arch Linux (PKGBUILD)](#arch-linux-pkgbuild)
  - [NixOS](#nixos)
  - [OpenWrt (router mode)](#openwrt-router-mode)
  - [Windows](#windows)
- [Usage](#usage)
  - [Interactive TUI](#interactive-tui)
  - [Command-line mode](#command-line-mode)
  - [Router mode](#router-mode)
- [Configuration (`conf.env`)](#configuration-confenv)
- [Services](#services)
- [Uninstalling](#uninstalling)
- [Project layout](#project-layout)
- [Troubleshooting](#troubleshooting)
- [License](#license)

---

## Features

- **Two engine generations**: the original `zapret` (Flowseal strategies) and `zapret2` (`git.zapret.moe` presets). Switch between them in the TUI or the config file.
- **Automatic downloads**: fetches `nfqws` / `winws` and `nfqws2` / `winws2` binaries and strategy bundles from GitHub and other sources.
- **Firewall integration**: sets up `NFQUEUE` rules with `nftables` (JSON API) or `iptables` on Linux, and uses `WinDivert` on Windows.
- **Safe teardown**: RAII guards remove redirection rules if launching or running the engine fails.
- **Hardened Linux staging**: engine binaries and configs are staged in `/run/zapret` (or `/tmp/zapret-runtime`) with restricted permissions. The engine gets `cap_net_admin` through `setcap`, so nothing runs setuid root.
- **Connectivity diagnostics**: probes targets with hand-crafted QUIC Initial packets and `curl` requests across predefined service bundles.
- **Autotune**: evaluates strategy effectiveness, builds a service availability matrix, and discovers TTL values.
- **Service integration**: runs as a background service under systemd, OpenRC, runit, dinit, s6, `init.d`, or procd (OpenWrt), or as a native Windows service.
- **Router mode**: headless operation on OpenWrt, with a procd init script and preflight checks.
- **Two ways to use it**: the interactive TUI, or a headless CLI for scripts and automation.

---

## Platform support

| OS | Firewall backend | Service management |
|---|---|---|
| **Linux (glibc or musl)** | `nftables` or `iptables` (`NFQUEUE`) | systemd, OpenRC, runit, dinit, s6, `init.d`, procd (OpenWrt) |
| **OpenWrt** | `nftables` (default on 22.03+) or `iptables` | procd (`/etc/init.d/zapret-rust`), or router mode (`-R`) |
| **Windows** | `WinDivert` (`winws` / `winws2`) | Native Windows service |

macOS and FreeBSD binaries can be downloaded, but firewall and service automation are not implemented for them.

---

## Quick start

On a typical Linux desktop, once the dependencies are installed (see [Requirements](#requirements)):

```bash
git clone https://github.com/soufied/zapret-tui-rs.git
cd zapret-tui-rs
cargo build --release
sudo ./target/release/zapret-rust
```

The first run downloads the engine and strategies, then opens the menu.

---

## Installation

### Requirements

**Runtime (Linux):**

- Root access through `sudo` or `pkexec` (polkit).
- A firewall backend: `nftables` (`nft`) or `iptables`.
- `curl`, used for connectivity checks and downloads.
- `setcap` (from libcap), to grant `cap_net_admin` to the engine.
- `pgrep` and `pkill` (from procps).

**Runtime (Windows):** run as Administrator.

**Build:**

- A recent stable Rust toolchain, installed with [rustup](https://rustup.rs) or from your distribution's packages.
- A C compiler and linker (`gcc` or `clang`), plus `pkg-config`.

The build needs no system libraries beyond the C toolchain.

### Build from source (any Linux)

```bash
git clone https://github.com/soufied/zapret-tui-rs.git
cd zapret-tui-rs
cargo build --release
```

The binary is written to `target/release/zapret-rust`. Copy it wherever you like, for example `/usr/local/bin/`. A plain `cargo build` produces a debug build, which is slower, so use `--release` for everyday use.

To produce a statically linked binary for portable deployment, build against the musl target:

```bash
rustup target add x86_64-unknown-linux-musl     # or aarch64-unknown-linux-musl
sudo apt install musl-tools                     # Debian/Ubuntu; the package name differs elsewhere
cargo build --release --target x86_64-unknown-linux-musl
```

The binary is then at `target/x86_64-unknown-linux-musl/release/zapret-rust`.

### Debian and Ubuntu

```bash
sudo apt update
sudo apt install build-essential pkg-config git curl \
  nftables iptables policykit-1 libcap2-bin

# Rust, if it is not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"

git clone https://github.com/soufied/zapret-tui-rs.git
cd zapret-tui-rs
cargo build --release
sudo ./target/release/zapret-rust
```

On Debian and Ubuntu, `policykit-1` provides `pkexec`, and `libcap2-bin` provides `setcap`.

### Fedora

```bash
sudo dnf install gcc pkgconf-pkg-config git curl \
  nftables iptables polkit libcap rust cargo

git clone https://github.com/soufied/zapret-tui-rs.git
cd zapret-tui-rs
cargo build --release
sudo ./target/release/zapret-rust
```

Distribution Rust packages can lag behind upstream. If the build fails with a compiler-version error, install Rust with rustup instead.

### Arch Linux (PKGBUILD)

A build script is included in `packaging/arch/PKGBUILD`. It builds from the tagged release source and installs the binary to `/usr/bin/zapret-rust`.

```bash
sudo pacman -S --needed base-devel git curl nftables iptables polkit libcap procps-ng
git clone https://github.com/soufied/zapret-tui-rs.git
cd zapret-tui-rs/packaging/arch
makepkg -si
```

The default cache directory is the directory of the executable, which would put downloads and `conf.env` into `/usr/bin`. Keep them elsewhere with `-d`:

```bash
sudo zapret-rust -d /var/lib/zapret-rust
```

### NixOS

The repository is a Nix flake. It provides a package, a development shell, an overlay, and a NixOS module.

**Prerequisites**, which must be done in the repository before Nix can build it:

```bash
cargo generate-lockfile          # creates Cargo.lock if it is missing
git add -A                       # flakes only see files tracked by git
nix flake lock                   # creates flake.lock
```

**Build and run without installing:**

```bash
nix build .#zapret-rust          # result/bin/zapret-rust
nix run .#zapret-rust            # run directly
```

**Development shell** (cargo, rustc, clippy, rust-analyzer):

```bash
nix develop
cargo build --release
```

**Install with the NixOS module.** Add the flake to your system configuration:

```nix
{
  inputs.zapret-rust.url = "github:soufied/zapret-tui-rs";

  outputs = { nixpkgs, zapret-rust, ... }: {
    nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        zapret-rust.nixosModules.default
        {
          services.zapret-rust = {
            enable = true;
            strategy = "discord.bat";     # a strategy file the engine provides
            # engine = "zapret";          # or "zapret2"
            # backend = "iptables";
            # interface = "any";
            # gamefilter.tcp = true;
            # dpiDesyncTtl = 6;
            # cacheDir = "/var/lib/zapret-rust";
          };
        }
      ];
    };
  };
}
```

Apply it with `sudo nixos-rebuild switch --flake .#my-host`.

The module writes `/etc/zapret-rust/conf.env`, runs a `zapret-rust` systemd service, and keeps downloaded binaries in `/var/lib/zapret-rust` (change this with `cacheDir`). On NixOS, use the module's service rather than the TUI's Service menu, because Nix manages `/etc/systemd/system`.

Notes:

- **Firewall reloads.** If you enable `networking.nftables`, reloading the ruleset flushes all tables, including zapret's. Restart the service after firewall changes with `sudo systemctl restart zapret-rust`.
- **Dynamic binaries.** The module enables `nix-ld` by default so that dynamically linked engine binaries can run. Set `services.zapret-rust.nixLd = false` if the downloaded binaries are static. Check with `ldd` on the binary in the cache directory.

### OpenWrt (router mode)

Router mode runs headless. It is meant for a router that should bypass blocks for every device on the LAN. OpenWrt images do not include a Rust toolchain, so build on a development machine and copy the binary to the router.

**1. Find the router's CPU and build for it.**

```bash
ssh root@192.168.1.1 'uname -m; cat /etc/openwrt_release'
```

Build with the matching musl target:

| Router CPU (`uname -m`) | Rust target | Notes |
|---|---|---|
| `x86_64` | `x86_64-unknown-linux-musl` | x86 routers and VMs |
| `aarch64` | `aarch64-unknown-linux-musl` | Most modern ARM routers |
| `armv7l` | `armv7-unknown-linux-musleabihf` | 32-bit ARM. Some boards need `arm-unknown-linux-musleabihf` instead |
| `mips` / `mipsel` | No prebuilt standard library | Needs nightly with `-Zbuild-std`, or a cross tool. Not verified |

```bash
rustup target add aarch64-unknown-linux-musl
cargo build --release --target aarch64-unknown-linux-musl
```

You can also use [`cross`](https://github.com/cross-rs/cross), which builds these targets in containers, or the OpenWrt SDK if you want to match the router's C library exactly. These steps have not been tested on hardware, so if one route fails, try another.

**2. Copy the binary to the router.**

```bash
scp target/aarch64-unknown-linux-musl/release/zapret-rust root@192.168.1.1:/usr/bin/zapret-rust
ssh root@192.168.1.1 chmod +x /usr/bin/zapret-rust
```

**3. Install the runtime packages on the router.** OpenWrt 25.x and newer use `apk`:

```bash
apk update
apk add nftables kmod-nft-queue curl libcap-bin
```

OpenWrt 24.10 and older use `opkg`:

```bash
opkg update
opkg install nftables kmod-nft-queue curl libcap-bin
```

To use the iptables backend, install `iptables-mod-nfqueue iptables-mod-extra` in place of the nftables packages. Package names can vary between releases. Search with `apk search <name>` or `opkg find <name>` if one is missing.

**4. Create the configuration.** Router mode reads `/usr/share/zapret-rust/conf.env` by default:

```bash
mkdir -p /usr/share/zapret-rust
cat > /usr/share/zapret-rust/conf.env << 'CONF_END'
engine=zapret
interface=any
strategy=discord.bat
gamefiltertcp=false
gamefilterudp=false
backend=nftables
CONF_END
```

Strategy names come from the engine bundle. Run `zapret-rust` (the TUI) once on the router, or use the CLI with `-R -s <strategy>`, to see what is available.

**5. Run it.** Start in the foreground first to check that it works:

```bash
zapret-rust -R
```

Then install it as a service. Run the TUI once on the router, open **Service**, and choose install. This writes `/etc/init.d/zapret-rust`, which you can manage with the standard OpenWrt commands:

```bash
/etc/init.d/zapret-rust start
/etc/init.d/zapret-rust stop
/etc/init.d/zapret-rust enable      # start at boot
```

**6. Turn off flow offloading.** Offloaded connections skip the netfilter hooks that zapret depends on, so bypass can fail silently. Router mode warns you if this is enabled. To disable it:

```bash
uci set firewall.@defaults[0].flow_offloading='0'
uci set firewall.@defaults[0].flow_offloading_hw='0'
uci commit firewall
/etc/init.d/firewall restart
```

Some router CPUs (for example, little-endian MIPS) need a manually chosen engine binary, because the downloader picks binaries by CPU architecture only.

### Windows

1. Install [Rust](https://rustup.rs) with the default MSVC toolchain. Visual Studio Build Tools with the C++ workload are required for linking.
2. Build from a terminal:

   ```powershell
   git clone https://github.com/soufied/zapret-tui-rs.git
   cd zapret-tui-rs
   cargo build --release
   ```

3. Run `target\release\zapret-rust.exe` as Administrator. The first run downloads the engine and strategies into the executable's directory.

To run as a Windows service, use the Service menu in the TUI. Remove it the same way.


---

## Usage

### Interactive TUI

Run the program without arguments to open the menu:

```bash
sudo ./zapret-rust                # Linux
```

On Windows, run the executable from an elevated terminal.

### Command-line mode

Pass options directly, or load a saved configuration file:

```bash
# Run a specific strategy on a specific interface
sudo ./zapret-rust -s discord.bat -i eth0

# Run with the game filter enabled for TCP and UDP
sudo ./zapret-rust -s discord.bat -t -u

# Load settings from a configuration file
sudo ./zapret-rust --config conf.env

# Use a custom cache directory
sudo ./zapret-rust -d /opt/zapret-rust --config conf.env
```

| Option | Description |
|---|---|
| `-c, --config <FILE>` | Load settings from a configuration file |
| `-s, --strategy <NAME>` | Strategy to run |
| `-i, --interface <NAME>` | Network interface (default: `any`) |
| `-t, --gamefiltertcp` | Enable the game filter for TCP |
| `-u, --gamefilterudp` | Enable the game filter for UDP |
| `-d, --cache-dir <DIR>` | Directory for downloaded binaries and strategies |
| `-R, --router` | Headless router mode (OpenWrt); see below |
| `-h, --help` | Show help |

Running with `-s` or `--config` skips the TUI and runs in the foreground. Press `Ctrl+C` to stop it.

### Router mode

`-R` (`--router`) runs headless, without the TUI. It:

- uses `/usr/share/zapret-rust` for config and downloads, unless `-d` or `ZAPRET_CACHE_DIR` is set;
- reads `conf.env` from that directory, unless `-c` is given;
- refuses to start without a strategy;
- prints preflight warnings for missing tools, with the install command for the router's package manager (`apk` or `opkg`), and for enabled flow offloading.

Router mode is intended for OpenWrt. See [OpenWrt](#openwrt-router-mode) for setup.

---

## Configuration (`conf.env`)

The configuration file is created and updated automatically in the application directory. Router and NixOS setups use the cache directory instead.

```ini
engine=zapret
interface=any
strategy=discord.bat
gamefiltertcp=false
gamefilterudp=false
backend=nftables
active_discord_fake=quic_initial_steamcommunity_com.bin
active_gamefilter_fake=quic_initial_4pda_to.bin
dpi_desync_ttl=
editor=
backup_lists=true
```

| Key | Description |
|---|---|
| `engine` | `zapret` (`nfqws` / `winws`) or `zapret2` (`nfqws2` / `winws2`) |
| `interface` | Network interface to bind rules to (`any` or a specific name) |
| `strategy` | Strategy or preset file to run |
| `gamefiltertcp` / `gamefilterudp` | `true` enables the game filter (zapret engine only) |
| `backend` | Linux firewall backend: `nftables` or `iptables` |
| `active_discord_fake` / `active_gamefilter_fake` | Fake packet payloads used by the active strategy (zapret engine only) |
| `dpi_desync_ttl` | TTL override for desync packets (empty for the strategy default) |
| `editor` | Editor command used to open strategy files |
| `backup_lists` | Whether list files are backed up (`true` / `false`) |

---

## Services

| Platform | Install | Start / stop | Remove |
|---|---|---|---|
| systemd | TUI → Service | `systemctl start zapret-rust` / `stop` | TUI → Service, or see [Uninstalling](#uninstalling) |
| OpenRC, runit, dinit, s6, `init.d` | TUI → Service | the init system's own commands | TUI → Service |
| OpenWrt (procd) | TUI → Service | `/etc/init.d/zapret-rust start` / `stop` | TUI → Service |
| NixOS | `services.zapret-rust.enable = true` | `systemctl` | Remove the module configuration |
| Windows | TUI → Service | Windows Services console | TUI → Service |

The zapret2 engine uses the service name `zapret2-rust`.

---

## Uninstalling

Remove the service first, then the files.

```bash
# Linux, if the service was installed from the TUI: open the TUI and choose Service → remove
sudo ./zapret-rust

# systemd, manually
sudo systemctl disable --now zapret-rust
sudo rm -f /etc/systemd/system/zapret-rust.service
sudo systemctl daemon-reload

# OpenWrt
/etc/init.d/zapret-rust stop
/etc/init.d/zapret-rust disable
rm -f /etc/init.d/zapret-rust
rm -rf /usr/share/zapret-rust /usr/bin/zapret-rust

# Arch Linux
sudo pacman -R zapret-rust
```

On NixOS, remove the `services.zapret-rust` block from your configuration and run `nixos-rebuild switch`.

Firewall rules are removed when the service stops. If rules are left over after a crash, start the service and stop it again, or run the TUI and stop from there.

---

## Project layout

```
src/
├── main.rs         Entry point and CLI
├── config.rs       conf.env parsing and engine selection
├── download.rs     Binary and strategy downloads
├── runner.rs       Process lifecycle
├── router.rs       Router mode (OpenWrt) checks and defaults
├── strategy/       Strategy discovery and parsing
├── autotune/       Probing, scoring, and TTL discovery
├── firewalls/      nftables, iptables, and WinDivert backends
├── inits/          Service integration per init system (including procd)
├── platform/       Linux and Windows specifics
└── tui/            ratatui interface and menus
locales/            English and Russian translations
packaging/arch/     Arch Linux PKGBUILD
nix/                Nix package and NixOS module
flake.nix           Nix flake (package, dev shell, overlay, module)
```

---

## Troubleshooting

- **"Binary or strategies missing"**: open the download menu in the TUI to fetch them. Check your network connection, or point `-d` at a directory that already contains them.
- **Rules not applied**: confirm the firewall backend is installed (`nft --version` or `iptables --version`) and that you are running as root.
- **Bypass has no effect on a router**: check that flow offloading is disabled (see [OpenWrt](#openwrt-router-mode)).
- **Service fails to start**: check the Log menu in the TUI. On systemd systems, `journalctl -u zapret-rust` (or `zapret2-rust` for the zapret2 engine) may help.
- **NixOS: the downloaded engine binary won't start**: confirm that `nixLd` is enabled, or set it to `false` if the binary is static.
- **Build fails with a compiler error**: update Rust with `rustup update stable`.

---

## License

MIT
