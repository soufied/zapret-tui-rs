# zapret-rust

A terminal (TUI) and CLI controller for both [**zapret**](https://github.com/bol-van/zapret) (`nfqws`/`winws`) and [**zapret2**](https://github.com/bol-van/zapret2) (`nfqws2`/`winws2`) by [bol-van](https://github.com/bol-van). It automates deep packet inspection (DPI) bypass configurations for blocked services like Discord and YouTube on Linux and Windows.

---

## Overview

Configuring packet desynchronization manually requires managing firewall rules, selecting strategies, and updating binaries. `zapret-rust` wraps this workflow into an interactive `ratatui` TUI and scriptable CLI:

* **Dual-generation engine support**: Supports both the original `zapret` (with Flowseal strategies) and `zapret2` (with `git.zapret.moe` presets).
* **Automated firewall management**: Installs and tears down rules via `nftables`/`iptables` (`NFQUEUE` 200) on Linux and `WinDivert` on Windows.
* **Diagnostics & Autotune**: Evaluates strategy effectiveness via QUIC and HTTP(S) probes, service availability matrices, and automatic TTL discovery.
* **System service integration**: Installs as a background daemon across Linux init systems (systemd, OpenRC, runit, dinit, s6, init.d) or native Windows services.

---

## Key Features

* **Engine & Preset Lifecycle**: Automatically downloads and version-tracks `nfqws`/`winws` and `nfqws2`/`winws2` binaries from GitHub alongside community strategy bundles.
* **Firewall Backends & Safety**: Supports `nftables` (JSON API) and `iptables` on Linux. Employs RAII guards to automatically remove redirection rules if launch or execution fails.
* **Security & Staging (Linux)**: Stages engine binaries and configurations in `/run/zapret` (or `/tmp/zapret-runtime`) with restricted permissions and applies `cap_net_admin` via `setcap` instead of running setuid root.
* **Connectivity Testing**: Tests target reachability using hand-crafted QUIC initial packets and `curl` probes across predefined service bundles (Discord, YouTube, etc.).
* **Dual Operation Modes**: Full-featured interactive terminal UI or a direct headless CLI for automation.

---

## Platform Support

| OS | Firewall Backend | Service Management |
|---|---|---|
| **Linux** | `nftables` or `iptables` (`NFQUEUE`) | systemd, OpenRC, runit, dinit, s6, `init.d` |
| **Windows** | `WinDivert` (`winws` / `winws2`) | Native Windows Service (`windows-service`) |

*(macOS and FreeBSD architectures are supported for binary downloads only; firewall and service automation are not implemented).*

---

## Dependencies & Installation

### Requirements
* **Rust**: 2021 edition toolchain.
* **Permissions**: Root (`sudo`/`pkexec`) on Linux, Administrator on Windows.
* **Linux runtime packages**: `nftables` (or `iptables`), `polkit` (or `sudo`), `libcap2-bin` (`setcap`), and `curl`.

```bash
# Debian / Ubuntu
sudo apt install build-essential pkg-config nftables iptables polkit libcap2-bin curl

# Arch Linux
sudo pacman -S base-devel nftables iptables polkit libcap curl
```

### Building

```bash
git clone https://github.com/soufied/zapret-tui-rs.git
cd zapret-tui-rs
cargo build --release
```
The compiled binary will be placed at `target/release/zapret-rust` (or `.exe` on Windows).

---

## Usage

### Interactive TUI
Launch without arguments to open the menu interface:
```bash
sudo ./zapret-rust
```


### CLI (Non-Interactive)
Launch directly with parameters or through a saved configuration file:
```bash
# Run with a specific strategy and network interface
sudo ./zapret-rust -s discord.bat -i eth0

# Run from a configuration file
sudo ./zapret-rust --config conf.env

# Use a custom cache directory
sudo ./zapret-rust -d /opt/zapret-rust --config conf.env
```


---

## Configuration (`conf.env`)

Automatically generated and updated in the executable directory:

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
| `engine` | Active engine: `zapret` (`nfqws`/`winws`) or `zapret2` (`nfqws2`/`winws2`) |
| `interface` | Network interface to bind rules to (`any` or specific name) |
| `strategy` | Strategy/preset script file to execute |
| `backend` | Linux firewall implementation (`nftables` or `iptables`) |
| `dpi_desync_ttl` | Manual TTL override for desync packets (leave empty for strategy default) |

---
