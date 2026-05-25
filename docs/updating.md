# Updating Drop

## Update from Your Fork (Forgejo → CT)

```bash
cd /opt/drop/src/drop
git fetch origin
git pull origin develop

cd server
pnpm install
pnpm run build
systemctl restart drop
```

## Pull Upstream Changes into Your Fork

```bash
cd /opt/drop/src/drop

# Fetch upstream
git fetch https://github.com/Drop-OSS/drop.git develop

# Merge (resolve conflicts if any)
git merge FETCH_HEAD --no-edit

# Regenerate CSS after upstream changes to Tailwind config or components
cd server
pnpm run build:css

# Rebuild
pnpm run build

# Push to both remotes
git push origin develop      # Forgejo
git push github develop      # GitHub fork

# Restart
systemctl restart drop
```

## Checking for Upstream Releases

```bash
git fetch https://github.com/Drop-OSS/drop.git --tags
git tag | sort -V | tail -5
```

## After Major Updates

If the Prisma schema changed, run migrations before restarting:

```bash
cd /opt/drop/src/drop/server
DATABASE_URL="postgres://drop:droppass@127.0.0.1:5432/drop" \
  npx prisma migrate deploy
```
