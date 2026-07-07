#!/usr/bin/env bash
set -euo pipefail

SERVICE="machenike-hotkeysd.service"
PURGE="false"

if [[ "${1:-}" == "--purge" ]]; then
  PURGE="true"
fi

if [[ "${EUID}" -ne 0 ]]; then
  echo "Run as root:"
  echo "  sudo ./scripts/uninstall.sh"
  echo
  echo "Full purge:"
  echo "  sudo ./scripts/uninstall.sh --purge"
  exit 1
fi

echo "[1/6] Stopping daemon..."
systemctl stop "${SERVICE}" 2>/dev/null || true
systemctl disable "${SERVICE}" 2>/dev/null || true
systemctl reset-failed "${SERVICE}" 2>/dev/null || true

echo "[2/6] Removing binaries..."
rm -f /usr/local/bin/machenike-kbdctl
rm -f /usr/local/bin/machenike-hotkeysd
rm -f /usr/local/bin/machenike-config

echo "[3/6] Removing systemd service..."
rm -f "/etc/systemd/system/${SERVICE}"
rm -f "/etc/systemd/system/multi-user.target.wants/${SERVICE}"

echo "[4/6] Reloading systemd..."
systemctl daemon-reload

if [[ "${PURGE}" == "true" ]]; then
  echo "[5/6] Purging config, state and acpi_call autoload..."
  rm -rf /etc/machenike
  rm -rf /var/lib/machenike-kbdctl
  rm -f /etc/modules-load.d/acpi_call.conf

  # Do not remove the acpi_call package. Only try to unload the module.
  modprobe -r acpi_call 2>/dev/null || true
else
  echo "[5/6] Keeping config and state..."
  echo "Keeping:"
  echo "  /etc/machenike"
  echo "  /var/lib/machenike-kbdctl"
  echo
  echo "For full removal use:"
  echo "  sudo ./scripts/uninstall.sh --purge"
fi

echo "[6/6] Done."

echo
echo "============================================================"
echo " MACHENIKE Linux uninstalled"
echo "============================================================"
echo
echo "Removed:"
echo "  /usr/local/bin/machenike-kbdctl"
echo "  /usr/local/bin/machenike-hotkeysd"
echo "  /usr/local/bin/machenike-config"
echo "  /etc/systemd/system/${SERVICE}"
echo

if [[ "${PURGE}" == "true" ]]; then
  echo "Purged:"
  echo "  /etc/machenike"
  echo "  /var/lib/machenike-kbdctl"
  echo "  /etc/modules-load.d/acpi_call.conf"
  echo
else
  echo "Config was kept."
  echo
fi
