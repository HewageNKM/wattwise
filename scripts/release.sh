#!/bin/bash
set -e

# Zenith Energy release script
# Purpose: Build, Version, Checksum, and Reinstall

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}==> Zenith Energy Release Tool${NC}"

# Check if script is run as root
if [ "$EUID" -eq 0 ]; then
  echo -e "${RED}Error: Please do not run this script as root/sudo.${NC}"
  echo -e "The script will ask for sudo password only when needed for installation."
  exit 1
fi

# 1. Versioning
CURRENT_VERSION=$(grep -m 1 "version =" Cargo.toml | cut -d '"' -f 2)
echo -e "${BLUE}Current Version: ${CURRENT_VERSION}${NC}"
read -p "Enter new version (or press enter to keep): " NEW_VERSION

if [ ! -z "$NEW_VERSION" ]; then
    echo -e "${BLUE}Bumping version to ${NEW_VERSION}...${NC}"
    sed -i "s/version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
    sed -i "s/\"version\": \"$CURRENT_VERSION\"/\"version\": \"$NEW_VERSION\"/" tauri.conf.json
    VERSION=$NEW_VERSION
else
    VERSION=$CURRENT_VERSION
fi

# 2. Build Dependencies Check (npm/cargo)
echo -e "${BLUE}==> Verifying environment...${NC}"
npm install

# 3. Comprehensive Build
echo -e "${BLUE}==> Building .deb package...${NC}"
npm run tauri build -- --bundles deb

# 4. SHA256 Checksum
DEB_FILE="src-tauri/target/release/bundle/deb/zenith-energy_${VERSION}_amd64.deb"
# Note: Tauri 2.0 might have different output paths, checking common locations
if [ ! -f "$DEB_FILE" ]; then
    DEB_FILE=$(find target/release/bundle/deb/ -name "*.deb" | head -n 1)
fi

if [ -f "$DEB_FILE" ]; then
    echo -e "${BLUE}==> Generating SHA256 checksum...${NC}"
    sha256sum "$DEB_FILE" > "${DEB_FILE}.sha256"
    echo -e "${GREEN}Checksum saved to ${DEB_FILE}.sha256${NC}"
else
    echo -e "${RED}Error: .deb file not found!${NC}"
    exit 1
fi

# 5. Reinstall
echo -e "${BLUE}==> Reinstalling package...${NC}"
sudo dpkg -i "$DEB_FILE"

# 6. Enable Service
echo -e "${BLUE}==> Setting up background daemon...${NC}"
echo -e "${BLUE}==> Disabling conflicting power managers...${NC}"
for svc in "power-profiles-daemon.service" "tlp.service" "thermald.service"; do
    if systemctl is-active --quiet "$svc"; then
        echo -e "${YELLOW}Stopping and masking $svc...${NC}"
        sudo systemctl stop "$svc" || true
        sudo systemctl mask "$svc" || true
    fi
done

sudo mkdir -p /etc/zenith-energy
sudo chmod 777 /etc/zenith-energy
sudo touch /var/log/zenith-energy.log
sudo chmod 644 /var/log/zenith-energy.log
sudo systemctl daemon-reload
sudo systemctl enable --now zenith-energy.service

echo -e "${GREEN}SUCCESS: Zenith Energy ${VERSION} installed and running.${NC}"
