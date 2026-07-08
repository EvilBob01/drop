# Install Guide — Drop OSS (Native, No Docker)

This guide documents the exact steps used to deploy Drop on **Ducky**
(Proxmox VE host, Debian 13 LXC container, no Docker). It is a reproducible
reference for a clean rebuild from scratch.

> **Quick links:** [Admin Guide](docs/admin-guide.md) · [Updating](docs/updating.md) · [Backup & Restore](docs/backup-restore.md) · [Packaging a .deb](docs/packaging.md)

---

## System Requirements

| Component | Minimum | Recommended |
|---|---|---|
| OS | Debian 12 (bookworm) | Debian 13 (trixie) |
| CPU | 2 cores | 4+ cores |
| RAM | 4 GB | 6 GB+ |
| Disk (OS + app) | 20 GB | 40 GB |
| Node.js | 22.16.0 | latest LTS 22.x |
| PostgreSQL | 15 | 17 |
| Rust toolchain | nightly | nightly |

Game storage lives on a separate volume (ZFS, NFS, etc.) and is mounted read-only.

---

## 1. Proxmox LXC Setup

Run the following on the **Proxmox host**:

```bash
# Download Debian 13 template if not already present
pveam update
pveam download local debian-13-standard_13.x-x_amd64.tar.zst

# Create the container (CT 108 in this example)
pct create 108 local:vztmpl/debian-13-standard_13.x-x_amd64.tar.zst \
  --hostname games \
  --cores 4 \
  --memory 6144 \
  --swap 1024 \
  --rootfs local-lvm:40 \
  --net0 name=eth0,bridge=vmbr0,ip=192.168.1.143/24,gw=192.168.1.1 \
  --nameserver 192.168.1.1 \
  --searchdomain 360pc.net \
  --features nesting=1 \
  --unprivileged 0

# Bind-mount the games library read-only into the container
echo "mp0: /voracity/incoming/Done/Games,mp=/library,ro=1" >> /etc/pve/lxc/108.conf

pct start 108
pct exec 108 -- bash
```

> **Privileged container** (`--unprivileged 0`) is required for the ZFS bind mount.
> The `nesting=1` feature is required for systemd to work correctly inside the CT.

---

## 2. Base System Packages

```bash
apt-get update && apt-get upgrade -y
apt-get install -y \
  curl git screen build-essential libssl-dev unzip gnupg2 \
  nginx postgresql postgresql-client fuser \
  protobuf-compiler pkg-config
```

---

## 3. Node.js 22

```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
apt-get install -y nodejs

# Enable pnpm via corepack (no separate install needed)
corepack enable
corepack prepare pnpm@9 --activate
```

Verify:
```bash
node --version   # v22.x.x
pnpm --version   # 9.x.x
```

---

## 4. Rust Nightly

Drop's `torrential` daemon and `droplet-rs` library require Rust nightly.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- --default-toolchain nightly -y
source /root/.cargo/env

rustup default nightly
rustc --version  # rustc 1.x.x-nightly
```

Add to `/root/.bashrc` so it persists across sessions:
```bash
echo 'source /root/.cargo/env' >> /root/.bashrc
```

---

## 5. PostgreSQL

```bash
systemctl enable --now postgresql

# Create the drop database user and database
sudo -u postgres psql << 'SQL'
CREATE USER drop WITH PASSWORD 'droppass';
CREATE DATABASE drop OWNER drop;
GRANT ALL PRIVILEGES ON DATABASE drop TO drop;
SQL
```

> Change `droppass` to a strong random password and update `.env` accordingly.

---

## 6. Optional: Tailscale

For remote access without exposing port 443 publicly:

```bash
curl -fsSL https://pkgs.tailscale.com/stable/debian/trixie.noarmor.gpg \
  | tee /usr/share/keyrings/tailscale-archive-keyring.gpg >/dev/null
curl -fsSL https://pkgs.tailscale.com/stable/debian/trixie.tailscale-keyring.list \
  | tee /etc/apt/sources.list.d/tailscale.list

apt-get update && apt-get install -y tailscale
tailscale up
```

---

## 7. Create the `drop` System User

```bash
useradd -r -m -d /opt/drop -s /bin/bash drop
mkdir -p /opt/drop/{src,logs}
chown -R drop:drop /opt/drop
```

---

## 8. Clone the Fork

```bash
mkdir -p /opt/drop/src
cd /opt/drop/src
git clone https://github.com/EvilBob01/drop.git drop
# or from Forgejo:
# git clone https://git.360pc.com/evilbob/drop.git drop

cd drop
git remote add github https://github.com/EvilBob01/drop.git 2>/dev/null || true
git checkout develop
```

---

## 9. Build Torrential (Rust P2P Daemon)

```bash
cd /opt/drop/src/drop

# Initialise submodules (droplet-rs)
git submodule update --init --recursive

cd torrential
cargo build --release 2>&1 | tail -10

# Install the binary system-wide
cp target/release/torrential /usr/local/bin/torrential
chmod +x /usr/local/bin/torrential
```

> **Rust nightly borrow-checker patch:** The `droplet-rs` submodule has a
> lifetime annotation patch applied in this fork for compatibility with
> Rust nightly ≥ 2026-05-24. It is already committed; no manual fix needed.

---

## 10. Build the Drop Server

```bash
cd /opt/drop/src/drop/server

# Install Tailwind CLI (required for the pre-build CSS step — the Vite plugin
# does not work in Nuxt SSR/Nitro builds)
pnpm add -D @tailwindcss/cli

# Install all workspace dependencies
NODE_OPTIONS=--max-old-space-size=8192 pnpm install

# Pre-generate Tailwind CSS (must run before nuxt build)
pnpm run build:css

# Build the server (Nuxt + Nitro bundle)
# NODE_OPTIONS prevents OOM on large monorepo builds
pnpm run build
```

The full build takes 5–10 minutes. Output lands in `server/.output/`.

---

## 11. Fake `cargo` Wrapper

Drop checks for the `../torrential/` source directory and falls into dev mode,
trying to run `cargo run` instead of the pre-built binary. This wrapper
intercepts that call:

```bash
cat > /usr/local/bin/cargo << 'EOF'
#!/bin/bash
# Intercept Drop's dev-mode torrential spawn; run the pre-built binary instead.
pkill -x torrential 2>/dev/null
sleep 2
exec /usr/local/bin/torrential
EOF
chmod +x /usr/local/bin/cargo
```

---

## 12. Environment File

Create `/opt/drop/src/drop/server/.env`:

```env
DATABASE_URL="postgres://drop:droppass@127.0.0.1:5432/drop"
EXTERNAL_URL="https://games.360pc.net"
PORT=4000
TORRENTIAL_PATH=/usr/local/bin/torrential

# Optional — IGDB metadata (requires free Twitch Developer account)
# Get keys at https://dev.twitch.tv/console
IGDB_CLIENT_ID=""
IGDB_CLIENT_SECRET=""
```

> `PORT=4000` is for the Nuxt Node.js process. Drop's embedded nginx takes
> port 3000 and proxies to it. Your external reverse proxy talks to port 3000.

---

## 13. Run Database Migrations

```bash
cd /opt/drop/src/drop/server
DATABASE_URL="postgres://drop:droppass@127.0.0.1:5432/drop" \
  pnpm exec prisma migrate deploy
```

---

## 14. Systemd Service

Create `/etc/systemd/system/drop.service`:

```ini
[Unit]
Description=Drop OSS Game Library Server
After=network.target postgresql.service
Requires=postgresql.service

[Service]
Type=simple
User=drop
Group=drop
WorkingDirectory=/opt/drop/src/drop/server
EnvironmentFile=/opt/drop/src/drop/server/.env
ExecStartPre=/bin/bash -c 'pkill -u drop nginx 2>/dev/null; sleep 1; fuser -k 3000/tcp 2>/dev/null || true'
ExecStart=/usr/bin/node .output/server/index.mjs
Restart=on-failure
RestartSec=10
KillMode=control-group
StandardOutput=append:/opt/drop/logs/drop.log
StandardError=append:/opt/drop/logs/drop-error.log

[Install]
WantedBy=multi-user.target
```

```bash
systemctl daemon-reload
systemctl enable --now drop
systemctl status drop
```

Watch the log:
```bash
tail -f /opt/drop/logs/drop.log
```

---

## 15. System Nginx Reverse Proxy

Drop's embedded nginx listens on **port 3000**. Put a system nginx in front
for SSL termination.

Install certbot:
```bash
apt-get install -y certbot python3-certbot-nginx
# For Namecheap DNS challenge:
pip install certbot-dns-namecheap  # or use --manual
```

Get the certificate:
```bash
certbot certonly --manual --preferred-challenges dns -d games.360pc.net
# Follow the DNS TXT record instructions
```

Create `/etc/nginx/sites-available/drop`:
```nginx
server {
    listen 80;
    server_name games.360pc.net;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl;
    server_name games.360pc.net;

    ssl_certificate     /etc/letsencrypt/live/games.360pc.net/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/games.360pc.net/privkey.pem;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         HIGH:!aNULL:!MD5;

    # Forward everything to Drop's embedded nginx
    location / {
        proxy_pass         http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header   Upgrade    $http_upgrade;
        proxy_set_header   Connection "upgrade";
        proxy_set_header   Host       $host;
        proxy_set_header   X-Real-IP  $remote_addr;
        proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Proto https;
        proxy_read_timeout 3600s;
        proxy_buffering    off;
    }
}
```

```bash
ln -s /etc/nginx/sites-available/drop /etc/nginx/sites-enabled/
nginx -t && systemctl reload nginx
```

---

## 16. First-Run Setup

1. Open `https://games.360pc.net` (or `http://192.168.1.143:3000` on LAN)
2. You will be directed to `/auth/setup` to create the admin account
3. After creating the account, sign in at `/auth/signin`
4. Go to **Admin → Library → Add source**, set path to `/library`
5. Use the **Auto-batch import** feature to bulk-import all games — see
   [Admin Guide → Auto-Batch Import](docs/admin-guide.md#auto-batch-import)

---

## Port Reference

| Port | Process | Purpose |
|---|---|---|
| 3000 | Drop embedded nginx | Public-facing HTTP |
| 4000 | Nuxt Node.js | App server (internal) |
| 5000 | Torrential depot | P2P file serving (internal) |
| 33148 | Torrential IPC | Internal control socket |
| 5432 | PostgreSQL | Database |

---

## Troubleshooting

**Service won't start — port 3000 in use:**
```bash
fuser -k 3000/tcp
systemctl restart drop
```

**Torrential crash loop (port 33148 conflict):**
The fake cargo wrapper kills the old torrential before starting a new one.
If it loops, check: `pgrep -a torrential`

**`EACCES: permission denied` on `.data/`:**
Ensure `WorkingDirectory` in the service file is `/opt/drop/src/drop/server`,
not `.output/server/`. Drop creates `.data/` relative to the working directory.

**OOM during build:**
```bash
NODE_OPTIONS=--max-old-space-size=8192 pnpm run build
```

**Blank pages / missing CSS:**
```bash
cd /opt/drop/src/drop/server
pnpm run build:css  # regenerate Tailwind
pnpm run build
systemctl restart drop
```

**Stuck metadata tasks:**
```sql
-- Mark stale in-progress tasks as failed so the import queue clears
UPDATE "Task"
SET success = false, error = '"Timed out"'::jsonb, ended = NOW()
WHERE progress < 100 AND success IS NULL AND ended IS NULL;
```

**PCGamingWiki cover art 403:**
Already fixed in this fork (`transactional.ts` User-Agent patch). If it
recurs after an upstream merge, verify the patch is still in place.
