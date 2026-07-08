# Admin Guide

## Accessing the Admin Dashboard

Navigate to `http://192.168.1.143:3000/admin` (LAN) or `https://games.360pc.net/admin` (public).
Sign in with your admin credentials.

---

## Managing the Library

### Adding the Library Source
1. Admin Dashboard → **Library** → **Add Library**
2. Set path to `/library` (bind-mounted from `/voracity/incoming/Done/Games`)
3. Drop will scan and list all folders as game candidates

### Importing Games
1. From the Library list, select games to import
2. Drop searches Steam and PCGamingWiki for matching metadata
3. If no match is found automatically, use the **Manual** provider to fill in details
4. Cover art and screenshots are downloaded and cached in `.data/data/objects/`

### Adding IGDB Metadata (optional)
1. Get a free Twitch Developer key at `https://dev.twitch.tv/console`
2. Add to `/opt/drop/src/drop/server/.env`:
   ```
   IGDB_CLIENT_ID=your_client_id
   IGDB_CLIENT_SECRET=your_client_secret
   ```
3. `systemctl restart drop`

---

## Managing Users

### Creating Invite Links
1. Admin Dashboard → **Users** → **Invite**
2. Copy the link and send to the family member
3. They register at the invite URL — no admin approval needed

### User Roles
- **Admin** — full access including library management
- **User** — browse store, download games, manage own library

### Resetting a Password
Admin Dashboard → **Users** → click user → **Reset Password**

---

## Logs

| File | Contents |
|---|---|
| `/opt/drop/logs/drop.log` | Main application log |
| `/opt/drop/logs/drop-error.log` | Node.js errors and crashes |
| `/opt/drop/logs/torrential.log` | P2P daemon log |

```bash
# Follow live:
tail -f /opt/drop/logs/drop.log | grep -v nginx
```


---

## Auto-Batch Import

Instead of importing games one-by-one through the UI, use the bulk import
endpoint that automatically matches all library games to metadata.

### How It Works

1. Reads every folder in `/library` not yet in the database
2. Normalises the folder name: strips `[FitGirl Repack]`, `-GOG`, `-TENOKE`,
   dot-separators, version strings, etc. via `gameNameNormalize`
3. Searches Steam and PCGamingWiki with the clean name
4. Queues a game-import task if the top match score is at or above the threshold
5. Returns a JSON report of queued vs skipped games

### Running It

Use the CLI API token stored in the database:

```bash
# Dry run — predictions without creating any tasks
curl -s -X POST http://localhost:3000/api/v1/admin/import/game/auto-batch \
  -H 'Authorization: Bearer drop-cli-autobatch-token-360pc' \
  -H 'Content-Type: application/json' \
  -d '{"dryRun": true, "minScore": 0.75}' | python3 -m json.tool

# Real run
curl -s -X POST http://localhost:3000/api/v1/admin/import/game/auto-batch \
  -H 'Authorization: Bearer drop-cli-autobatch-token-360pc' \
  -H 'Content-Type: application/json' \
  -d '{"minScore": 0.75}' | python3 -m json.tool
```

### Parameters

| Parameter | Default | Description |
|---|---|---|
| `dryRun` | `false` | If true, report only — no tasks created |
| `minScore` | `0.75` | Minimum fuzzy match confidence 0 to 1 |
| `gameType` | `"Game"` | GameType to assign all imported games |
| `limit` | `0` | Max games per call; 0 = unlimited |

### Skipped Games

Games that do not reach the confidence threshold appear in the `skipped` array.
Review them manually at **Admin Library Import**. Common reasons:
- ROM packs and BIOS files
- Emulator bundles
- Obscure titles with no Steam or PCGamingWiki entry

### Re-running After Adding New Games

Run the same command again. Games already imported or with active tasks are
automatically excluded from each run.
