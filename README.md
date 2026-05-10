# ADDITION

> Free, open-source desktop game manager. Mods, trainers, automatic detection — all launchers, no paywalls.

ADDITION blends CurseForge-style mod browsing with WeMod-style trainer access into a single dark-themed Tauri desktop app. It auto-detects games from Steam, Epic Games, GOG, EA, Ubisoft Connect, Xbox Game Pass and Rockstar Launcher, snapshots target folders before any change, and never sends telemetry.

## Features

- **Auto-detection** across seven launchers — zero configuration on install
- **Mod browser** powered by CurseForge & Nexus Mods (bring your own free API keys)
- **One-click install / uninstall** with automatic per-install backups
- **Curated trainer index** with explicit single-player-only disclaimers and anti-cheat warnings
- **Local-only** SQLite library — no accounts, no cloud sync, no telemetry
- **Cross-platform** Tauri build targets (.msi, .dmg, AppImage)

## Tech stack

| Layer    | Tooling                                                   |
| -------- | --------------------------------------------------------- |
| Frontend | React 18 + TypeScript + Vite, plain CSS Modules           |
| Backend  | Rust (Tauri v2) — scanners, mod installer, backup engine  |
| Storage  | SQLite via `tauri-plugin-sql`                             |
| HTTP     | `reqwest` (Rust) + `fetch` (frontend, for browser CORS)   |

## Repo layout

```
ADDITION/
├── src/                      React frontend
│   ├── api/                  invoke() wrappers, SQLite client, mod APIs
│   ├── components/           Sidebar, Topbar, GameCard, ToastContainer, …
│   ├── pages/                LibraryPage, GameDetailPage, SettingsPage, …
│   ├── pages/tabs/           ModsTab, TrainersTab, InstalledTab, GameSettingsTab
│   ├── store/                Zustand stores (library, toast)
│   └── styles/globals.css    Design tokens + Lato @font-face
├── src-tauri/                Rust backend
│   ├── src/scanner/          One file per launcher (steam, epic, gog, …)
│   ├── src/mods/             install.rs, backup.rs (zip snapshots)
│   ├── src/db/                SQLite migrations + app data dir
│   └── src/commands.rs       Tauri command surface
└── public/assets/            Logo, Lato fonts
```

## Setup

### Prerequisites
- **Node.js** 18+
- **Rust** 1.77+ (`rustup install stable`)
- **Tauri prerequisites** — see <https://tauri.app/start/prerequisites/>
  - **Windows**: Microsoft C++ Build Tools, WebView2 runtime
  - **macOS**: Xcode CLT
  - **Linux**: webkit2gtk, libssl, etc.

### Install dependencies

```bash
npm install
cd src-tauri && cargo fetch && cd ..
```

### Run in development

```bash
npm run tauri:dev
```

This starts Vite on `http://localhost:1420` and launches the Tauri window pointing at it. Hot reload works for both the React app and (on save + rebuild) the Rust backend.

### Build a release artifact

```bash
npm run tauri:build
```

Output: `src-tauri/target/release/bundle/`.

## API keys & sign-in

End users do **not** copy/paste API keys.

| Service     | UX for end-user                            | How it works                                                  |
| ----------- | ------------------------------------------ | ------------------------------------------------------------- |
| CurseForge  | Just works.                                | App ships with a build-time API key (see *Bundling…* below).  |
| Nexus Mods  | One click → "Sign in with Nexus".          | Websocket SSO flow (Vortex / MO2 use the same protocol).      |
| Cover art   | Just works.                                | Free Steam CDN; falls back to Steam search by name.           |

### Bundling the CurseForge key (maintainers only)

CurseForge's ToS allows distributing apps with a developer-issued key.

1. Generate one at <https://console.curseforge.com>.
2. Copy `.env.example` to `.env.local`.
3. Set `VITE_CURSEFORGE_API_KEY=<your-key>` and rebuild. The key is inlined into the bundled JS at build time.
4. *Optional but recommended:* host a Cloudflare Worker proxy at e.g. `mods.addition.app/curseforge/*` that injects the header server-side, so the key never lives in the client bundle. Update [src/api/mods.ts](src/api/mods.ts) to point at the proxy URL.

### Registering the Nexus SSO slug (maintainers only)

The slug `addition` is hard-coded in [src/api/nexusSso.ts](src/api/nexusSso.ts). Before the SSO flow works for end users, register the slug at <https://www.nexusmods.com/users/myaccount?tab=api> — pick a unique application name, paste the resulting slug into the constant in `nexusSso.ts`. Until that's done, the sign-in button will fail with an "application not registered" error from Nexus.

### Per-user advanced overrides

Settings → **Show advanced overrides** still exposes manual fields for users who want to use their own CurseForge or SteamGridDB keys (for higher rate limits or curated grid covers).

## How detection works

| Launcher  | Source of truth                                                       |
| --------- | --------------------------------------------------------------------- |
| Steam     | `steamapps/libraryfolders.vdf` + each `appmanifest_*.acf`              |
| Epic      | `C:\ProgramData\Epic\…\LauncherInstalled.dat`                          |
| GOG       | `HKLM\SOFTWARE\GOG.com\Games`                                          |
| EA/Origin | `HKLM\SOFTWARE\WOW6432Node\Electronic Arts` (and Origin Games)         |
| Ubisoft   | `HKLM\SOFTWARE\Ubisoft\Launcher\Installs`                              |
| Xbox      | Folders directly under `C:\XboxGames\`                                 |
| Rockstar  | `HKLM\SOFTWARE\WOW6432Node\Rockstar Games\…`                           |

Each scanner is best-effort: a missing launcher cannot break the rest. Manually-added folders sit alongside detected games and use the `manual` launcher tag.

## Mod install pipeline

1. **Download** the archive into `{app_data}/staging/{game_id}/`.
2. **Snapshot** the target folder into `{app_data}/backups/{game_id}/{backup_id}.zip` (deflate-compressed).
3. **Extract** the archive into the target folder.
4. **Record** the install + backup ID in SQLite so the **Installed** tab can roll back later.

Uninstall == restore-from-snapshot, then drop the row. No mod ever bypasses the backup step.

## Trainers

The MVP ships a small static trainer index pointing to free sources (FLiNG Trainers etc.). Anti-cheat games are auto-flagged and the launch button is disabled in the UI. **Single-player only — always.**

## Privacy

ADDITION makes outbound HTTPS requests **only** to:
- **Steam's public CDN** when fetching cover art (no auth, no account)
- **Steam's storefront search** when matching non-Steam games to a cover (no auth)
- **Nexus' SSO server** during the one-time sign-in handshake
- **Mod source APIs** (CurseForge, Nexus) when the user opens a mod browser

There is no analytics, no crash reporter, no auto-update channel, no remote config. Read the entire `src-tauri/src/` and `src/api/` trees to verify.

## Build order (for contributors)

1. Tauri shell compiles & launches → blank window.
2. Game scanner returns rows → library grid renders cards.
3. Game detail page → tabs.
4. Mods tab → CurseForge / Nexus integration.
5. Trainer tab → static catalogue.
6. Polish: cover art, drag-and-drop load order, conflict detection.

## License

MIT — see `LICENSE`. Logo and Lato font files are bundled under their respective licenses (see `local font files/OFL.txt` for Lato).
