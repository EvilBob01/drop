<div align="center">
<img src="https://raw.githubusercontent.com/Drop-OSS/media-sources/refs/heads/main/drop.svg" width="200rem"/>
</div>
<br/>

# Drop — 360pc Fork

[![Upstream](https://img.shields.io/badge/upstream-Drop--OSS%2Fdrop-blue?style=for-the-badge)](https://github.com/Drop-OSS/drop)
[![Based on](https://img.shields.io/badge/based%20on-v0.4.0-green?style=for-the-badge)](https://github.com/Drop-OSS/drop/releases/tag/v0.4.0)
[![GitHub License](https://img.shields.io/badge/AGPL--3.0-red?style=for-the-badge)](LICENSE)
[![Install](https://img.shields.io/badge/INSTALL%20GUIDE-black?style=for-the-badge)](INSTALL.md)

Self-hosted DRM-free game distribution for families — no Docker required.

This is [EvilBob01](https://github.com/EvilBob01)'s fork of [Drop OSS](https://github.com/Drop-OSS/drop),
patched for native Debian/Proxmox deployment and extended with bulk-import tooling for large FitGirl/GOG libraries.

<div align="center">
<img src="https://droposs.org/_ipx/f_webp&q_80/images/carousel/store.png" alt="Drop Screenshot" width="900rem"/>
</div>

---

## What This Fork Adds

| Feature | Description |
|---|---|
| **No Docker** | Runs natively on Debian 12/13 as a systemd service |
| **Debian package** | Pre-built `.deb` available on the [Releases](https://github.com/EvilBob01/drop/releases) page |
| **Game name normaliser** | Strips `[FitGirl Repack]`, `-GOG`, `-TENOKE`, dot-separators, and version strings before metadata search |
| **Auto-batch import** | One API call imports all 500+ games from `/library` with fuzzy metadata matching |
| **Image carousel fix** | Screenshots from Steam/PCGamingWiki now populate the game detail carousel |
| **PCGamingWiki fix** | Browser User-Agent headers added to image proxy — no more 403 on cover art |
| **Tailwind CSS fix** | Pre-generates CSS via `@tailwindcss/cli` to work around broken Vite plugin in Nuxt SSR builds |

---

## Quick Install (Debian .deb)

Download the latest `.deb` from the [Releases](https://github.com/EvilBob01/drop/releases) page and install it on any Debian 12/13 amd64 machine:

```bash
# Download the latest release
wget https://github.com/EvilBob01/drop/releases/latest/download/drop-oss_amd64.deb

# Install (PostgreSQL and nginx must already be installed)
apt-get install -y postgresql nginx nodejs
dpkg -i drop-oss_amd64.deb

# Configure
cp /opt/drop/app/.env.example /opt/drop/app/.env
nano /opt/drop/app/.env   # set DATABASE_URL and EXTERNAL_URL

# Start
systemctl start drop
```

Then open `http://localhost:3000` to complete first-run setup.

---

## Manual Install (from source)

See **[INSTALL.md](INSTALL.md)** for the full step-by-step guide covering:

- Proxmox LXC container setup
- Node.js 22, Rust nightly, PostgreSQL 17
- Building Torrential (Rust P2P daemon) from source
- Building the Nuxt server with Tailwind CSS pre-generation
- Systemd service, nginx reverse proxy, Let's Encrypt SSL
- First-run setup and initial game import

---

## Bulk Game Import

After adding your library source in the admin panel, import all games in one shot:

```bash
# Dry run first — see what will be matched
curl -s -X POST http://localhost:3000/api/v1/admin/import/game/auto-batch \
  -H 'Authorization: Bearer YOUR_API_TOKEN' \
  -H 'Content-Type: application/json' \
  -d '{"dryRun": true, "minScore": 0.75}' | python3 -m json.tool

# Run it for real
curl -s -X POST http://localhost:3000/api/v1/admin/import/game/auto-batch \
  -H 'Authorization: Bearer YOUR_API_TOKEN' \
  -H 'Content-Type: application/json' \
  -d '{"minScore": 0.75}' | python3 -m json.tool
```

The normaliser handles common repack naming conventions automatically:

| Library folder | Search query sent to Steam |
|---|---|
| `A.Plague.Tale.Requiem.v1076-GOG` | `A Plague Tale Requiem` |
| `Age of Empires IV [FitGirl Repack]` | `Age of Empires IV` |
| `Above.Snakes-TENOKE` | `Above Snakes` |
| `RimWorld.v1.4.3613-GOG` | `RimWorld` |

---

## Documentation

| Document | Description |
|---|---|
| [INSTALL.md](INSTALL.md) | Full native install guide (no Docker) |
| [CHANGELOG.md](CHANGELOG.md) | Fork patches + upstream release history |
| [docs/admin-guide.md](docs/admin-guide.md) | Day-to-day admin tasks, auto-batch import |
| [docs/user-guide.md](docs/user-guide.md) | End-user guide for the store and downloads |
| [docs/updating.md](docs/updating.md) | How to update the server and pull upstream changes |
| [docs/backup-restore.md](docs/backup-restore.md) | Backup and restore procedures |
| [docs/packaging.md](docs/packaging.md) | How to build your own `.deb` package |

---

## Upstream

This fork tracks [Drop-OSS/drop](https://github.com/Drop-OSS/drop) on the `develop` branch.
Upstream bug fixes are cherry-picked regularly. See [CHANGELOG.md](CHANGELOG.md) for the merge log.

---

## Philosophy (from upstream)

1. **Flexible** — abstractions and interfaces are worth the complexity.
2. **Secure** — never accessible without authentication; supports SSO, 2FA, OIDC.
3. **User-friendly** — clean interface with advanced features available when needed.

---

## License

AGPL-3.0 — see [LICENSE](LICENSE).
