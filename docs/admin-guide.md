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
