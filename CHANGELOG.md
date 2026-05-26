# Changelog

All notable changes to this fork (EvilBob01/drop) are documented here.
Upstream changes from [Drop-OSS/drop](https://github.com/Drop-OSS/drop) are also summarised.

---

## [Fork] 360pc Deployment — May 2026

Custom patches and configuration applied to the EvilBob01/drop fork for
self-hosted deployment on Ducky (Proxmox, 96 GB RAM, 32 cores, voracity ZFS pool).

### Changed
- `server/tailwind.config.js` — added `../libraries/base/**` content paths so
  the base-layer Vue components are scanned by Tailwind
- `server/assets/styles.css` — added explicit `@source` directives for monorepo
  paths to ensure Tailwind v4 oxide scanner finds all class usage
- `server/assets/generated.css` — pre-generated Tailwind CSS via CLI (183 KB);
  bypasses the broken `@tailwindcss/vite` SSR-build pipeline in Nuxt 3
- `server/nuxt.config.ts` — switched CSS import to `generated.css`, disabled
  `experimental.buildCache` to prevent stale CSS being served from cache, removed
  `tailwindcss()` Vite plugin (replaced by CLI pre-generation step)
- `server/package.json` — build script now runs `tailwindcss -i … -o …` before
  `nuxt build`; added `NODE_OPTIONS=--max-old-space-size=8192` to prevent Nitro
  OOM on large monorepo builds
- `libraries/droplet-rs` — patched `ArchiveReader` lifetime borrow-checker error
  that prevented compilation on Rust nightly ≥ 2026-05-24
- `server/.env` — `PORT=4000` (Nuxt), `TORRENTIAL_PATH=/usr/local/bin/torrential`

### Fixed
- `server/server/internal/objects/transactional.ts` — added browser `User-Agent`
  and `Referer` headers to image proxy fetches; PCGamingWiki CDN was returning
  403 Forbidden for server-side requests without a recognisable browser UA

### Infrastructure
- Deployed as Proxmox LXC CT 108 (Debian 13, 6 GB RAM, 4 cores, 40 GB rootfs)
- Library bind-mounted read-only: `/voracity/incoming/Done/Games` → `/library`
- Fake `cargo` wrapper at `/usr/local/bin/cargo` redirects Drop's dev-mode
  torrential spawn to the pre-compiled binary, avoiding source builds at runtime
- Systemd units for `drop` and `torrential` with `KillMode=control-group`

---

## [Upstream] v0.4.0 — May 2026

Major release converting Drop to a pnpm monorepo and introducing torrential v2.

### Added
- Monorepo structure: `server/`, `desktop/`, `torrential/`, `libraries/`
- Torrential v2 P2P engine (Rust) replacing the previous download daemon
- Prisma 7 with multi-file schema support
- Nuxt 3 + Tailwind CSS v4 frontend
- OpenAPI schema generation via Nitro
- WebSocket support via Nitro experimental flag
- Scheduled tasks system (`dailyTasks` cron)

### Changed
- Build system migrated from yarn to pnpm workspaces
- Node.js minimum version raised to 22.16.0
- PostgreSQL schema: 118 migrations applied

### Fixed
- Monorepo build pipeline (#404)
- Proxy buffering disabled for large file transfers (#5bbe406e)
- Z-index fix for ComboboxOptions (#375)

---

## [Upstream] v0.3.x — 2024–2025

- Initial public releases establishing core game library, metadata providers
  (Steam, PCGamingWiki, IGDB), user management, and torrential v1 P2P
