# ADDITION

> Your own game trainers. Free, open source, no accounts, no time limits.

ADDITION is a WeMod-style trainer app for **single-player** games. It finds your installed games, attaches to the one you're playing, and gives you toggles, number boxes and hotkeys for cheats like unlimited health or setting your money.

The difference is that the trainers are yours. Each one is a small JSON file, and the app has a built-in memory scanner (like Cheat Engine), so you can make new cheats without writing code.

## What's in it

- **Trainer tab** (per game). It waits for the game to start, attaches automatically, and gives you:
  - toggles to freeze a value ("Unlimited Health")
  - number boxes to set a value once or lock it ("Set Gold")
  - code patches that switch game logic off and back on ("No Damage")
  - global hotkeys (F1, Ctrl+F2, …) that work while the game has focus
- **Scanner tab**: find a value by searching, changing it in game and searching again. Then test it (edit or freeze), find a pointer path that survives restarts, and **Save as cheat**.
- **Game detection** across Steam, Epic, GOG, EA, Ubisoft, Xbox and Rockstar, plus manually added folders.
- **Safety**:
  - It refuses to attach while kernel anti-cheat (EAC, BattlEye, Vanguard, …) is running or loaded in the game.
  - Code patches are put back when you turn them off, close the trainer, or quit the app.
- **Works with 32-bit and 64-bit games** from the same 64-bit app.

**Single-player only.** Don't use trainers in online games. It gets accounts banned, and it's unfair to other players.

## Getting started

1. Build and run the app (see [Development](#development)), or grab the Windows installer from the latest CI run's artifacts.
2. Try the included test game first. It's a tiny console program built for practice:
   ```bash
   cargo run -p addition-engine --bin test-game -- --install-trainer   # writes its trainer into ADDITION's folder
   cargo run -p addition-engine --bin test-game -- --auto              # the "game": loses health every second
   ```
   In ADDITION, use **+ Add Manually**, pick `target/debug`, and name it "ADDITION Test Game". Open it and flip **Unlimited Health**.
3. Then try a real game. [docs/first-trainer.md](docs/first-trainer.md) walks through Plants vs. Zombies, which loads in seconds and is the classic game for learning this.

## Trainer files

Trainers live in `%APPDATA%\io.addition.app\trainers\*.json`. Settings → *Open trainers folder* takes you there.

- Bundled trainers ship inside the app ([src-tauri/trainers/](src-tauri/trainers/)).
- A file of yours with the same `id` replaces the bundled one. That's how saving a cheat into a bundled trainer works.

```jsonc
{
  "id": "my-game",                    // file name must be <id>.json
  "game": "My Game",                  // matched against your library
  "aliases": ["My Game: Deluxe"],
  "process": ["MyGame.exe"],          // attaches to the first one found
  "game_version": "Steam 1.4.2",
  "verified": true,
  "cheats": [
    { "id": "hp", "name": "Unlimited Health", "hotkey": "F1",
      "kind": "freeze", "type": "f32", "value": 100,
      "target": { "module": "MyGame.exe", "base": "0x1A2B3C", "offsets": ["0x18", "0x2C0"] } },

    { "id": "gold", "name": "Set Gold", "hotkey": "F2",
      "kind": "set", "type": "i32", "min": 0, "max": 999999, "default": 50000,
      "target": { "module": "MyGame.exe", "base": "0x1A2B40", "offsets": ["0x10"] } },

    { "id": "nodmg", "name": "No Damage", "hotkey": "F3",
      "kind": "patch", "aob": "29 47 ?? 8B 45 FC", "offset": 0, "bytes": "90 90 90" }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `kind: freeze` | Toggle. Keeps `target` at `value`, rewriting it every 50 ms. |
| `kind: set` | Number box with *Set* (write once) and a lock toggle. The hotkey writes `default`. |
| `kind: patch` | Toggle. Finds the byte signature `aob` (`??` = any byte) in `module` (default: the game's .exe), writes `bytes` at `offset` from the match, and restores the original bytes when turned off. Signatures usually survive game updates; fixed addresses don't. |
| `type` | `u8`, `i16`, `i32`, `i64`, `f32` or `f64`. |
| `target` | A pointer chain, `[["module"+base]+off1]+off2`. It reads a pointer at module base + `base`, adds `off1`, reads another pointer, adds `off2`. With no `module`, `base` is an absolute address (only good until the game restarts). |

## Layout

```
engine/                     Rust crate, no UI: everything that touches game memory
  src/sys/windows.rs        OpenProcess/Read/WriteProcessMemory, VirtualQueryEx, Toolhelp + psapi
  src/sys/linux.rs          /proc backend so the engine can be tested anywhere
  src/scan.rs               value scanner (exact / unknown / changed / increased / …)
  src/ptrscan.rs            pointer-path finder + re-check after restart
  src/aob.rs, pointer.rs    byte signatures, pointer chains
  src/session.rs            an attached trainer: freeze thread, patches with restore
  src/trainer.rs            the JSON trainer format
  src/anticheat.rs          refuses to attach around kernel anti-cheat
  src/bin/test-game.rs      practice "game" used by the tests
  tests/                    end-to-end tests against the real test-game process
src-tauri/                  Tauri app: commands, trainer host + hotkeys, launcher scanners
  trainers/                 bundled trainers
src/                        React UI (Library, Trainer tab, Scanner tab, Settings)
```

## Development

Prerequisites: Node 18+, Rust (stable), and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) (on Windows: C++ Build Tools + WebView2).

```bash
npm install
npm run tauri:dev            # app with hot reload
npm run tauri:build          # installer in target/release/bundle/
cargo test -p addition-engine
```

The engine tests start the real test game and do everything a user would: scan, narrow down, write, freeze, patch and restore, and find pointer paths across a restart. CI runs them on Linux and on Windows, against both a 64-bit and a 32-bit build of the test game, and builds the Windows installer.

## Privacy

There is no telemetry, no account and no time limit. The only outbound requests are for cover art (Steam's public CDN and store search, or SteamGridDB if you add a key).

## License

MIT, see `LICENSE`. The UI uses the Afterglow design system. Its typefaces (Unbounded, Manrope and JetBrains Mono) are bundled from Fontsource under the SIL Open Font License.
