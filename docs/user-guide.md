# User Guide

## Getting Access

Ask the admin (evilbob) for an invite link. Open it in your browser and
register with a username and password. No email verification needed.

---

## Browsing the Store

Navigate to `https://games.360pc.net` and sign in. The store shows all games
in the library with cover art, descriptions, and release dates.

Use the search bar or browse by tag to find games.

---

## Downloading a Game

1. Open a game page
2. Click **Add to Library** (adds it to your personal list)
3. Click **Download** — the Drop desktop client handles the transfer
4. Games download directly from the server using the torrential P2P engine

---

## Installing the Desktop Client

Download the Drop desktop app from the Releases page of the GitHub fork:
`https://github.com/EvilBob01/drop-app/releases`

On first launch, enter the server URL: `https://games.360pc.net`
Sign in with your credentials.

---

## On the LAN

For faster speeds on the local network, use the LAN address directly:
`http://192.168.1.143:3000`

---

## Troubleshooting

**Page won't load** — check that you're on the LAN or connected via Tailscale.

**Download is slow** — the server rate-limits per-user by default. Ask the admin
to check torrential settings.

**Game won't launch** — Drop downloads the installer (FitGirl repack or GOG
setup). Run the installer manually after downloading.
