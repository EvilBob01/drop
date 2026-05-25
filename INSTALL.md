# Install Guide — Drop OSS (Native, No Docker)

This guide documents the exact procedure used to deploy Drop on **Ducky**
(Proxmox, Debian 13 LXC, no Docker). It is a reproducible reference for
rebuilding from scratch.

---

## Prerequisites

| Requirement | Version | Notes |
|---|---|---|
| Proxmox VE | 8.x | Host hypervisor |
| Debian | 13 (trixie) | LXC template |
| Node.js | ≥ 22.16.0 | Via NodeSource repo |
| pnpm | 9.x | Via corepack |
| Rust | nightly | Via rustup |
| PostgreSQL | 16+ | System package |
| Nginx | any | Reverse proxy |

---

## 1. Create the LXC Container

On the Proxmox host:

```bash
# Download Debian 13 template
pveam download local debian-13-standard_13.x-x_amd64.tar.zst

# Create CT
pct create 108 local:vztmpl/debian-13-standard_13.x-x_amd64.tar.zst \
  --hostname games \
  --cores 4 \
  --memory 6144 \
  --swap 1024 \
  --rootfs VM:40 \
  --net0 name=eth0,bridge=vmbr0,ip=192.168.1.143/24,gw=192.168.1.1 \
  --nameserver 192.168.1.1 \
  --searchdomain 360pc.net \
  --features nesting=1 \
  --unprivileged 0

# Add games library bind mount (read-only)
echo "mp0: /voracity/incoming/Done/Games,mp=/library,ro=1" >> /etc/pve/lxc/108.conf

pct start 108
```

---

## 2. Base System Setup

```bash
pct exec 108 -- bash
apt-get update && apt-get upgrade -y
apt-get install -y curl git screen build-essential libssl-dev unzip gnupg2 \
  nginx postgresql postgresql-client protobuf-compiler
```

### Node.js 22

```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
apt-get install -y nodejs
corepack enable
corepack prepare pnpm@9 --activate
```

### Rust nightly

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- --default-toolchain nightly -y
source /root/.cargo/env
```

### PostgreSQL

```bash
systemctl enable --now postgresql
su - postgres -c "psql -c \"CREATE USER drop WITH PASSWORD 'droppass';\""
su - postgres -c "psql -c \"CREATE DATABASE drop OWNER drop;\""
su - postgres -c "psql -c \"GRANT ALL PRIVILEGES ON DATABASE drop TO drop;\""
```

### Tailscale

```bash
curl -fsSL https://pkgs.tailscale.com/stable/debian/trixie.noarmor.gpg \
  | tee /usr/share/keyrings/tailscale-archive-keyring.gpg >/dev/null
curl -fsSL https://pkgs.tailscale.com/stable/debian/trixie.tailscale-keyring.list \
  | tee /etc/apt/sources.list.d/tailscale.list
apt-get update && apt-get install -y tailscale
tailscale up
```

### drop system user

```bash
useradd -r -m -d /opt/drop -s /bin/bash drop
mkdir -p /opt/drop/{src,app,data,logs}
chown -R drop:drop /opt/drop
```

---

## 3. Clone the Fork

```bash
cd /opt/drop/src
git clone https://git.360pc.com/evilbob/drop.git drop
cd drop
git remote add github https://github.com/EvilBob01/drop.git
```

---

## 4. Build Torrential (Rust P2P daemon)

```bash
cd /opt/drop/src/drop
git submodule update --init --recursive
cd torrential
cargo build --release 2>&1 | tail -5
cp target/release/torrential /usr/local/bin/torrential
```

**Note:** The `droplet-rs` submodule requires a lifetime patch on newer Rust
nightly. See `CHANGELOG.md` for details — the patch is committed to this fork.

---

## 5. Build the Drop Server

```bash
cd /opt/drop/src/drop/server

# Install @tailwindcss/cli for pre-build CSS generation
pnpm add -D @tailwindcss/cli

# Install all dependencies
NODE_OPTIONS=--max-old-space-size=8192 pnpm install

# Generate Tailwind CSS (bypasses broken vite plugin in Nuxt SSR build)
pnpm run build:css

# Build the server
pnpm run build
```

The build takes ~5 minutes. Output lands in `.output/`.

---

## 6. Configure Environment

Create `/opt/drop/src/drop/server/.env`:

```env
DATABASE_URL="postgres://drop:droppass@127.0.0.1:5432/drop"
EXTERNAL_URL="https://games.360pc.net"
PORT=4000
TORRENTIAL_PATH=/usr/local/bin/torrential

# Optional — add for IGDB metadata (free Twitch Developer account)
IGDB_CLIENT_ID=""
IGDB_CLIENT_SECRET=""
```

---

## 7. Run Database Migrations

```bash
cd /opt/drop/src/drop/server
DATABASE_URL="postgres://drop:droppass@127.0.0.1:5432/drop" \
  npx prisma migrate deploy
```

---

## 8. Create fake cargo wrapper

Drop detects the `../torrential` source directory and tries to run `cargo run`
(dev mode). This wrapper intercepts that call and runs the pre-built binary:

```bash
cat > /usr/local/bin/cargo << 'EOF'
#!/bin/bash
pkill -x torrential 2>/dev/null
sleep 2
exec /usr/local/bin/torrential
EOF
chmod +x /usr/local/bin/cargo
```

---

## 9. Systemd Services

### Drop server (`/etc/systemd/system/drop.service`)

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
```

---

## 10. Nginx Reverse Proxy (system nginx)

Configure `/etc/nginx/sites-available/games` for HTTPS:

```nginx
server {
    listen 443 ssl;
    server_name games.360pc.net;

    ssl_certificate     /etc/letsencrypt/live/games.360pc.net/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/games.360pc.net/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}

server {
    listen 80;
    server_name games.360pc.net;
    return 301 https://$host$request_uri;
}
```

---

## 11. SSL — Let's Encrypt (Namecheap DNS challenge)

```bash
apt-get install -y certbot python3-certbot-dns-cloudflare
# Use Namecheap API plugin or manual DNS challenge:
certbot certonly --manual --preferred-challenges dns -d games.360pc.net
```

---

## 12. Updating Drop

```bash
cd /opt/drop/src/drop
git fetch origin && git pull origin develop
cd server
pnpm install
pnpm run build
systemctl restart drop
```

To pull upstream changes from Drop-OSS into the fork:

```bash
git fetch https://github.com/Drop-OSS/drop.git develop
git merge FETCH_HEAD
# resolve conflicts, then:
git push origin develop
git push github develop
```
