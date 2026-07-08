# Backup & Restore

## What Needs Backing Up

| Data | Location | Size | Notes |
|---|---|---|---|
| PostgreSQL database | Proxmox host backup or pg_dump | ~50 MB | All game metadata, users, sessions |
| Object storage (cover art) | `/opt/drop/src/drop/server/.data/` | ~2-8 GB | Re-downloadable from metadata providers |
| Environment config | `/opt/drop/src/drop/server/.env` | tiny | Contains secrets — back up securely |
| Game files | `/Games` | ~ALL The TB | Source of truth|

The ZFS pool (`poolname`) should be snapshotted regularly at the Proxmox level.
The game files themselves are never modified by Drop.

---

## PostgreSQL Backup

```bash
# On the CT (or via pct exec 108):
su - postgres -c "pg_dump drop" > /opt/drop/backups/drop-$(date +%Y%m%d).sql

# Restore:
su - postgres -c "psql drop < /opt/drop/backups/drop-20260525.sql"
```

---

## Object Storage Backup

```bash
tar -czf /opt/drop/backups/objects-$(date +%Y%m%d).tar.gz \
  /opt/drop/src/drop/server/.data/data/objects/
```

Cover art can be re-fetched from Steam/PCGamingWiki if lost, but a backup
saves the re-scraping time.

---

## Full CT Backup

Use Proxmox's built-in backup (vzdump) to snapshot the entire CT:

```bash
vzdump contaner# --storage local --compress zstd --mode snapshot
```

---

## Restore from Scratch

1. Re-run the install guide (`INSTALL.md`)
2. Restore the PostgreSQL dump
3. Restore `.data/` object storage
4. Copy back `.env`
5. `systemctl start drop`
