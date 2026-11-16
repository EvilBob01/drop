# Changelog

All notable changes to this fork (EvilBob01/drop) are documented here.
Upstream changes from [Drop-OSS/drop](https://github.com/Drop-OSS/drop) are also summarised.

---

## [Fork] Upstream sync — November 2026

Merged 68 commits from Drop-OSS/drop `develop` (up to `2e66ef9d`). All fork
patches carried over unchanged.

### Upstream highlights (server)
- User groups with age-rating restrictions (#454, #468) and age ratings on games (#451)
- Playtime tracking (#506)
- PKCE support for OIDC sign-in (#513)
- Store sort/filter kept in the URL (#536); header dropdown and favicon fixes (#535, #537)
- Steam import no longer fails when `full_description` is missing (#481)
- Object storage stream leak fixes and age-rating migration repair (#497)
- Autofill launch name on version import (#447); admin user bar overlap fix (#439)
- Torrential: archive backend crash fix (#475), dependency security bumps
- Nuxt 3.21.10, simplewebauthn 13.3.2, pino 9.14

### Database
- 3 new migrations: `repair_age_rating_schema`, `add_user_groups`,
  `add_version_game_id_indexes` (apply with `prisma migrate deploy`)

### Breaking for this deployment
- **PCGamingWiki metadata now requires a MediaWiki bot password**
  (`PCGAMINGWIKI_BOT_USERNAME` / `PCGAMINGWIKI_BOT_PASSWORD`, upstream #509).
  Without them the provider is skipped; Steam continues to work.

### Kept from fork
- Prisma stays at 7.8.0 (upstream pins 7.7.0)

---

## [Fork] 360pc Deployment — July 2026

### Added
- **Game name normalisation** (`server/internal/utils/gameNameNormalize.ts`)
  Strips release-group tags (`[FitGirl Repack]`, `-GOG`, `-TENOKE`, `-PROPHET`,
  `-PLAZA`, etc.), version strings (`v1.4.3613`, `v1076`, `1.0.0.0`), and expands
  dot-separated names (`A.Plague.Tale.Requiem` → `A Plague Tale Requiem`).
  Covers 30+ known repack groups.

- **Auto-batch import endpoint** (`POST /api/v1/admin/import/game/auto-batch`)
  Iterates every unimported game in the library, normalises its name, queries all
  metadata providers, and queues a game-import task for any result whose fuzzy
  confidence score meets the threshold (default 0.75). Supports `dryRun`,
  `minScore`, `gameType`, and `limit` parameters. Used to import 389 of 599 games
  in one pass. See `docs/admin-guide.md` for usage.

- **Normalised search endpoint** (`server/api/v1/admin/import/game/search.get.ts`)
  Normalises the `?q=` query before hitting Steam / PCGamingWiki / IGDB.

- **`normalizedName` field on unimported game list** — import UI pre-fills the
  search box with the clean name instead of the raw folder name.

- **Image carousel populated from metadata screenshots**
  (`server/internal/metadata/index.ts`) — `mImageCarouselObjectIds` is now set
  equal to `metadata.images` on game creation. Previously it was always empty
  despite screenshots being fetched into `mImageLibraryObjectIds`.
  SQL backfill applied for 380 existing games (7,830 objects already on disk).

- **System API token for CLI operations**
  CLI API token stored in `APIToken` table with
  `import:game:*` and `import:version:*` ACLs for headless admin scripts.

- **Debian package build infrastructure** (`packaging/`)
  `build-deb.sh` script and `debian/` control files for producing a self-contained
  `.deb` release package. See `docs/packaging.md`.

### Changed
- Import UI now pre-fills the search box with `normalizedName` instead of raw
  folder name when a game is selected for import.

### Fixed
- `server/server/internal/objects/transactional.ts` — added browser `User-Agent`
  and `Referer` headers to image proxy fetches; PCGamingWiki CDN was returning
  403 Forbidden for server-side requests without a recognisable browser UA.

- `server/server/internal/metadata/index.ts` — `mImageCarouselObjectIds` not
  being populated on game creation (upstream oversight).

---

## [Fork] 360pc Deployment — May/June 2026

Custom patches and configuration applied to the EvilBob01/drop fork for
self-hosted deployment on Ducky (Proxmox, 96 GB RAM, 32 cores, voracity ZFS pool).

### Changed
- `server/tailwind.config.js` — added `../libraries/base/**` content paths
- `server/assets/styles.css` — added explicit `@source` directives for monorepo paths
- `server/assets/generated.css` — pre-generated Tailwind CSS via CLI (183 KB);
  bypasses the broken `@tailwindcss/vite` SSR-build pipeline in Nuxt 3
- `server/nuxt.config.ts` — switched CSS import to `generated.css`, disabled
  `experimental.buildCache`, removed `tailwindcss()` Vite plugin
- `server/package.json` — build script runs `tailwindcss -i … -o …` before
  `nuxt build`; `NODE_OPTIONS=--max-old-space-size=8192` added to prevent OOM
- `libraries/droplet-rs` — patched `ArchiveReader` lifetime borrow-checker error
  blocking compilation on Rust nightly ≥ 2026-05-24
- `server/.env` — `PORT=4000` (Nuxt), `TORRENTIAL_PATH=/usr/local/bin/torrential`

### Infrastructure
- Proxmox LXC CT 108 (Debian 13, 6 GB RAM, 4 cores, 40 GB rootfs, privileged)
- Library bind-mounted read-only: `/voracity/incoming/Done/Games` → `/library`
- Fake `cargo` wrapper at `/usr/local/bin/cargo` redirects Drop's dev-mode
  torrential spawn to the pre-compiled binary, avoiding source builds at runtime
- Systemd unit `drop.service` with `KillMode=control-group`

---

## [Upstream] v0.4.0 — June 2026 (official release June 29, 2026)

Major release converting Drop to a pnpm monorepo and introducing torrential v2.

### Added
- Monorepo structure: `server/`, `desktop/`, `torrential/`, `libraries/`
- Torrential v2 P2P engine (Rust) replacing the previous download daemon
- Prisma 7 with multi-file schema support
- Nuxt 3 + Tailwind CSS v4 frontend
- OpenAPI schema generation via Nitro
- WebSocket support via Nitro experimental flag
- Scheduled tasks system (`dailyTasks` cron)
- Emulation support (Tauri desktop client)
- Two-factor authentication (2FA)
- Game update system
- In-app store
- Enhanced Proton / Wine configuration (desktop client)
- Whitelabeling support

### Changed
- Build system migrated from yarn to pnpm workspaces
- Node.js minimum version raised to 22.16.0
- PostgreSQL schema: 118 migrations applied

### Fixed
- Monorepo build pipeline (#404)
- Proxy buffering disabled for large file transfers (#5bbe406e)
- Z-index fix for ComboboxOptions (#375)
- SSR auth header forwarding in user composable (#414)
- Admin sidebar mobile layout (#268)
- Library source deletion UI (#269)
- Process handler and launch fixes (desktop client, #430)
- Local path and templating issues (desktop client, #436)

---

## [Upstream] v0.3.x — 2024–2025

- Initial public releases establishing core game library, metadata providers
  (Steam, PCGamingWiki, IGDB), user management, and torrential v1 P2P
