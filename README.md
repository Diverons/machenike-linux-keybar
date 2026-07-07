# machenike-linux

Linux keyboard backlight tools for MACHENIKE laptops.

This project started as reverse engineering work for the MACHENIKE Star 16 / V265RNX platform.  
It provides keyboard RGB control, configurable hotkeys, a background daemon, and a CLI configuration menu.

## Current status

Working:

- 3-zone keyboard RGB control
- Static colors
- Per-zone RGB colors
- Software brightness control
- Toggle on/off
- Color cycling
- Smooth RGB scroll while holding a hotkey
- Configurable hotkeys
- CLI configuration menu
- systemd daemon
- Install script
- Uninstall script

Tested on:

```text
Manufacturer: MACHENIKE
Product Name: Star 16
SKU: V265RNX
Board: V265RNX-HM
BIOS Vendor: INSYDE Corp.
BIOS Version: 1.07.03RQLM5
EC Firmware Revision: 9.2
```

## Requirements

The current implementation uses `acpi_call`.

On Arch Linux:

```bash
yay -S acpi_call-dkms
```

Or install the matching `acpi_call` package for your distribution.

Rust is required to build the project.

Arch Linux:

```bash
sudo pacman -S rust cargo
```

Or via rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Build

```bash
cargo build --release
```

## Install

```bash
sudo ./scripts/install.sh
```

The installer:

- checks that `cargo` is available
- builds release binaries
- installs binaries to `/usr/local/bin`
- installs the default config to `/etc/machenike/hotkeysd.conf`
- installs the systemd service
- enables `acpi_call` autoload
- starts the daemon

Installed binaries:

```text
/usr/local/bin/machenike-kbdctl
/usr/local/bin/machenike-hotkeysd
/usr/local/bin/machenike-config
```

## Uninstall

Keep config and state:

```bash
sudo ./scripts/uninstall.sh
```

Full removal:

```bash
sudo ./scripts/uninstall.sh --purge
```

Full purge removes:

```text
/etc/machenike
/var/lib/machenike-kbdctl
/etc/modules-load.d/acpi_call.conf
```

It does not remove the `acpi_call` package itself.

## CLI menu

```bash
sudo machenike-config
```

The menu allows you to:

- change RGB scroll speed
- change RGB hold delay
- configure hotkeys
- restore default settings
- restart daemon
- check service status

## Keyboard control

Static colors:

```bash
sudo machenike-kbdctl red
sudo machenike-kbdctl green
sudo machenike-kbdctl blue
sudo machenike-kbdctl white
sudo machenike-kbdctl off
```

Custom RGB:

```bash
sudo machenike-kbdctl rgb 255 120 0
```

Per-zone RGB:

```bash
sudo machenike-kbdctl zone 1 255 0 0
sudo machenike-kbdctl zone 2 0 255 0
sudo machenike-kbdctl zone 3 0 0 255
```

Brightness:

```bash
sudo machenike-kbdctl brightness 50
sudo machenike-kbdctl brightness-up
sudo machenike-kbdctl brightness-down
```

Color cycle:

```bash
sudo machenike-kbdctl next
```

Toggle:

```bash
sudo machenike-kbdctl toggle
```

Status:

```bash
sudo machenike-kbdctl status
```

## Default hotkeys

```text
Ctrl+Alt+/      color cycle / RGB scroll
Ctrl+Alt+*      toggle on/off
Ctrl+Alt+-      brightness down
Ctrl+Alt++      brightness up
```

Short press on the color hotkey switches to the next color.

Holding the color hotkey starts smooth RGB scrolling.

## Hotkey configuration

Open the CLI menu:

```bash
sudo machenike-config
```

Then choose:

```text
3) Hotkeys
```

When recording a hotkey:

```text
Press the desired key combination.
Then release it and do not press anything for 2 seconds.
The last captured combination will be saved.
```

Examples:

```text
ctrl+alt+slash
ctrl+alt+k
ctrl+shift+m
alt+space
meta+f
ctrl+alt+code:53
```

## Restore defaults

Open the CLI menu:

```bash
sudo machenike-config
```

Choose:

```text
4) Restore defaults
```

Then press Enter. The tool restores the default RGB scroll settings and default hotkeys.

You can also restore defaults directly:

```bash
sudo machenike-config defaults
```

## Config file

Main config:

```text
/etc/machenike/hotkeysd.conf
```

Default config:

```ini
# MACHENIKE hotkey daemon config

rgb_hold_delay_ms=300
rgb_step_delay_ms=18
rgb_hue_step=5

key_color=ctrl+alt+slash
key_toggle=ctrl+alt+8
key_brightness_down=ctrl+alt+minus
key_brightness_up=ctrl+alt+equal
```

Notes:

- `*` on the main keyboard is usually `Shift+8`
- default toggle is stored as `ctrl+alt+8`
- `+` on the main keyboard is usually `Shift+=`
- default brightness-up is stored as `ctrl+alt+equal`

The daemon also accepts aliases for main keyboard and numpad keys:

```text
slash      / on main keyboard
kpslash    / on numpad
minus      - on main keyboard
kpminus    - on numpad
equal      = / + on main keyboard
kpplus     + on numpad
8          8 / * on main keyboard
kpasterisk * on numpad
```

## Service

Check status:

```bash
systemctl status machenike-hotkeysd.service
```

Restart:

```bash
sudo systemctl restart machenike-hotkeysd.service
```

Logs:

```bash
journalctl -u machenike-hotkeysd.service -f
```

Show recent logs:

```bash
journalctl -u machenike-hotkeysd.service -n 30 --no-pager
```

## Reverse engineered EC protocol

ACPI method:

```text
\_SB.PC00.LPCB.EC.ECMD
```

Command format:

```text
{0x05, 0x00, 0xCA, zone, B, R, G, 0x00}
```

Zones:

```text
0x03 = left
0x04 = middle
0x05 = right
```

Important: color byte order is `B, R, G`.

Examples:

```text
White  {0x05, 0x00, 0xCA, zone, 0xC8, 0xC8, 0xC8, 0x00}
Blue   {0x05, 0x00, 0xCA, zone, 0xC8, 0x00, 0x00, 0x00}
Red    {0x05, 0x00, 0xCA, zone, 0x00, 0xC8, 0x00, 0x00}
Green  {0x05, 0x00, 0xCA, zone, 0x00, 0x00, 0xC8, 0x00}
```

## Project structure

```text
machenike-linux/
├── config/
│   └── hotkeysd.conf
├── crates/
│   ├── machenike-kbdctl/
│   ├── machenike-hotkeysd/
│   └── machenike-config/
├── scripts/
│   ├── install.sh
│   └── uninstall.sh
├── systemd/
│   └── machenike-hotkeysd.service
├── Cargo.toml
├── README.md
└── LICENSE
```

## Development notes

This is not a kernel driver yet.  
The current implementation is userspace-based and uses `acpi_call`.

The daemon currently reads `/dev/input/event*` directly and reacts to configured key combinations.

Possible future work:

- kernel/platform driver
- `/sys/class/leds` integration
- OpenRGB integration
- Fn hotkey support
- touchpad toggle support
- microphone mute toggle
- support for more MACHENIKE/Clevo-like models

## License

MIT
