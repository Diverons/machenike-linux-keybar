#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_USER="${SUDO_USER:-}"

if [[ "${EUID}" -ne 0 ]]; then
  echo "Run as root:"
  echo "  sudo ./scripts/install.sh"
  exit 1
fi

cd "${PROJECT_ROOT}"

echo "[0/7] Checking cargo..."

if [[ -n "${BUILD_USER}" && "${BUILD_USER}" != "root" ]]; then
  if ! sudo -u "${BUILD_USER}" bash -lc "command -v cargo >/dev/null 2>&1"; then
    echo "cargo not found for user: ${BUILD_USER}"
    echo
    echo "Install Rust first:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo
    echo "Or on Arch:"
    echo "  sudo pacman -S rust cargo"
    exit 1
  fi
else
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo not found"
    echo
    echo "Install Rust first:"
    echo "  sudo pacman -S rust cargo"
    exit 1
  fi
fi

echo "[1/7] Building release binaries..."

if [[ -n "${BUILD_USER}" && "${BUILD_USER}" != "root" ]]; then
  sudo -u "${BUILD_USER}" bash -lc "cd '${PROJECT_ROOT}' && cargo build --release"
else
  cargo build --release
fi

echo "[2/7] Installing binaries..."
install -m 755 target/release/machenike-kbdctl /usr/local/bin/machenike-kbdctl
install -m 755 target/release/machenike-hotkeysd /usr/local/bin/machenike-hotkeysd
install -m 755 target/release/machenike-config /usr/local/bin/machenike-config

echo "[3/7] Installing config..."
install -d -m 755 /etc/machenike

if [[ ! -f /etc/machenike/hotkeysd.conf ]]; then
  install -m 644 config/hotkeysd.conf /etc/machenike/hotkeysd.conf
else
  echo "Config already exists: /etc/machenike/hotkeysd.conf"
  echo "Keeping existing config."
fi

echo "[4/7] Installing systemd service..."
install -m 644 systemd/machenike-hotkeysd.service /etc/systemd/system/machenike-hotkeysd.service

echo "[5/7] Enabling acpi_call autoload..."
echo acpi_call > /etc/modules-load.d/acpi_call.conf

echo "[6/7] Loading acpi_call..."
modprobe acpi_call || true

echo "[7/7] Enabling and starting daemon..."
systemctl daemon-reload
systemctl enable --now machenike-hotkeysd.service
systemctl start machenike-hotkeysd.service

echo
echo "============================================================"
echo " MACHENIKE Linux installation completed"
echo "============================================================"
echo
echo "Keyboard backlight tools were installed successfully."
echo
echo "CLI menu:"
echo "  sudo machenike-config"
echo
echo "Basic commands:"
echo "  sudo machenike-kbdctl white"
echo "  sudo machenike-kbdctl next"
echo "  sudo machenike-kbdctl toggle"
echo "  sudo machenike-kbdctl brightness-up"
echo "  sudo machenike-kbdctl brightness-down"
echo
echo "Daemon service:"
echo "  systemctl status machenike-hotkeysd.service"
echo
echo "Logs:"
echo "  journalctl -u machenike-hotkeysd.service -f"
echo
echo "Config file:"
echo "  /etc/machenike/hotkeysd.conf"
echo
