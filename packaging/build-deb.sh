#!/bin/bash
# build-deb.sh — Build a self-contained .deb package for Drop OSS (360pc fork)
#
# Usage:
#   ./packaging/build-deb.sh [VERSION]
#
# Requirements (Debian/Ubuntu build host):
#   apt-get install -y dpkg-dev fakeroot
#
# What it does:
#   1. Builds the Drop server (Nuxt + Nitro)
#   2. Builds the Torrential daemon (Rust)
#   3. Copies built output into the debian package tree
#   4. Runs dpkg-deb to produce drop-oss_VERSION_amd64.deb

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VERSION="${1:-0.4.0-360pc1}"
PKG_NAME="drop-oss_${VERSION}_amd64"
BUILD_DIR="$SCRIPT_DIR/build/$PKG_NAME"

echo "==> Building Drop OSS $VERSION"

# ── 1. Build Torrential ──────────────────────────────────────────────────────
echo "==> Building torrential (Rust)..."
cd "$REPO_ROOT/torrential"
cargo build --release
TORRENTIAL_BIN="$REPO_ROOT/torrential/target/release/torrential"

# ── 2. Build Drop Server ──────────────────────────────────────────────────────
echo "==> Building Drop server (Node.js/Nuxt)..."
cd "$REPO_ROOT/server"
NODE_OPTIONS=--max-old-space-size=8192 pnpm install
pnpm run build:css
pnpm run build

# ── 3. Assemble Package Tree ──────────────────────────────────────────────────
echo "==> Assembling package..."
rm -rf "$BUILD_DIR"
cp -r "$SCRIPT_DIR/debian" "$BUILD_DIR"

# Update version in control file
sed -i "s/^Version:.*/Version: $VERSION/" "$BUILD_DIR/DEBIAN/control"

# App files: pre-built server output only (no sources/node_modules)
mkdir -p "$BUILD_DIR/opt/drop/app"
cp -r "$REPO_ROOT/server/.output" "$BUILD_DIR/opt/drop/app/output"

# Write example .env
cat > "$BUILD_DIR/opt/drop/app/.env.example" << 'ENV'
DATABASE_URL="postgres://drop:CHANGE_ME@127.0.0.1:5432/drop"
EXTERNAL_URL="https://your-domain.example.com"
PORT=4000
TORRENTIAL_PATH=/usr/local/bin/torrential
IGDB_CLIENT_ID=""
IGDB_CLIENT_SECRET=""
ENV

# Torrential binary
cp "$TORRENTIAL_BIN" "$BUILD_DIR/usr/local/bin/torrential"
chmod +x "$BUILD_DIR/usr/local/bin/torrential"

# Fix service file paths to match packaged layout
sed -i 's|WorkingDirectory=.*|WorkingDirectory=/opt/drop/app|' \
    "$BUILD_DIR/etc/systemd/system/drop.service"
sed -i 's|ExecStart=.*|ExecStart=/usr/bin/node /opt/drop/app/output/server/index.mjs|' \
    "$BUILD_DIR/etc/systemd/system/drop.service"
sed -i 's|EnvironmentFile=.*|EnvironmentFile=/opt/drop/app/.env|' \
    "$BUILD_DIR/etc/systemd/system/drop.service"
sed -i 's|StandardOutput=.*|StandardOutput=append:/opt/drop/logs/drop.log|' \
    "$BUILD_DIR/etc/systemd/system/drop.service"
sed -i 's|StandardError=.*|StandardError=append:/opt/drop/logs/drop-error.log|' \
    "$BUILD_DIR/etc/systemd/system/drop.service"

# Set correct permissions
find "$BUILD_DIR" -type d -exec chmod 755 {} \;
find "$BUILD_DIR" -type f -exec chmod 644 {} \;
chmod 755 "$BUILD_DIR/DEBIAN/postinst"
chmod 755 "$BUILD_DIR/DEBIAN/prerm"
chmod 755 "$BUILD_DIR/usr/local/bin/torrential"

# ── 4. Build the .deb ────────────────────────────────────────────────────────
echo "==> Building .deb..."
mkdir -p "$SCRIPT_DIR/build"
cd "$SCRIPT_DIR/build"
fakeroot dpkg-deb --build "$PKG_NAME" "${PKG_NAME}.deb"

echo ""
echo "==> Done: $SCRIPT_DIR/build/${PKG_NAME}.deb"
echo "    Install with: dpkg -i $SCRIPT_DIR/build/${PKG_NAME}.deb"
