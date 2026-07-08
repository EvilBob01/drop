# Packaging Drop OSS as a .deb

This document describes how to build a self-contained Debian package
(`drop-oss_VERSION_amd64.deb`) from this fork.

> The `.deb` packages a pre-compiled build, not sources. You build it once
> on a Debian machine, then `dpkg -i` it on any compatible target.

---

## What the Package Contains

| Path | Contents |
|---|---|
| `/opt/drop/app/output/` | Pre-built Nuxt/Nitro server bundle (`.output/`) |
| `/opt/drop/app/.env.example` | Template environment file |
| `/usr/local/bin/torrential` | Pre-compiled Rust P2P daemon |
| `/usr/local/bin/cargo` | Fake cargo wrapper (created by postinst) |
| `/etc/systemd/system/drop.service` | Systemd unit |

The package does **not** include sources, `node_modules`, or PostgreSQL data.

---

## Build Requirements

Run the build on a Debian 12/13 amd64 machine with:

```bash
# Build tools
apt-get install -y dpkg-dev fakeroot build-essential

# Node.js 22
curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
apt-get install -y nodejs
corepack enable && corepack prepare pnpm@9 --activate

# Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain nightly -y
source ~/.cargo/env
```

---

## Building the Package

```bash
cd /path/to/drop-repo

# Build with default version (0.4.0-360pc1)
./packaging/build-deb.sh

# Build with explicit version
./packaging/build-deb.sh 0.4.1-360pc1
```

The script will:
1. Run `cargo build --release` in `torrential/`
2. Run `pnpm run build` in `server/` (including Tailwind CSS pre-generation)
3. Assemble the package tree under `packaging/build/drop-oss_VERSION_amd64/`
4. Run `fakeroot dpkg-deb --build` to produce the `.deb`

Output: `packaging/build/drop-oss_VERSION_amd64.deb`

---

## Installing the Package

On the target server (Debian 12 or 13):

```bash
# Install dependencies first
apt-get install -y nodejs postgresql nginx

# Install the package
dpkg -i drop-oss_0.4.0-360pc1_amd64.deb

# Copy and edit the environment file
cp /opt/drop/app/.env.example /opt/drop/app/.env
nano /opt/drop/app/.env   # set DATABASE_URL, EXTERNAL_URL, passwords

# Start the service
systemctl start drop
```

---

## Publishing a GitHub Release

After building, attach the `.deb` to a GitHub release:

```bash
# Tag the release
git tag v0.4.0-360pc1
git push github v0.4.0-360pc1

# Create release and upload .deb via GitHub CLI
gh release create v0.4.0-360pc1 \
  --repo EvilBob01/drop \
  --title "Drop OSS 0.4.0 (360pc build 1)" \
  --notes "Native no-Docker build for Debian 12/13 amd64. See INSTALL.md." \
  packaging/build/drop-oss_0.4.0-360pc1_amd64.deb
```

Then install from a release on any machine:
```bash
VERSION="v0.4.0-360pc1"
wget "https://github.com/EvilBob01/drop/releases/download/${VERSION}/drop-oss_0.4.0-360pc1_amd64.deb"
dpkg -i drop-oss_0.4.0-360pc1_amd64.deb
```

---

## Version Numbering

Follow this scheme: `UPSTREAM_VERSION-360pcN`

- `0.4.0-360pc1` — first 360pc build based on upstream v0.4.0
- `0.4.0-360pc2` — same upstream base, fork patches updated
- `0.4.1-360pc1` — new upstream release

---

## Notes

- The Nitro server bundle is ~34 MB gzip. The full `.deb` will be ~70–100 MB
  including the torrential binary.
- The package does **not** run database migrations automatically. Run
  `prisma migrate deploy` manually or add it to `postinst` after testing.
- Game data (the `/library` bind mount) is never included in the package.
