# Your first real trainer: Plants vs. Zombies

Plants vs. Zombies is a good first target. It's tiny, it loads in a few seconds, it's offline and has no anti-cheat, and the Sun counter is a plain number on screen. People have been learning memory editing on it for years.

ADDITION ships a trainer for it built from community-published addresses. **Nobody has confirmed those addresses on your copy yet**, which is why the trainer shows an "Unverified" banner. Part 1 tests it. If it doesn't work, part 2 makes a working one with the Scanner in a few minutes. Part 2 is also how you'll make trainers for any other game.

## Part 1: try the bundled trainer

1. Install **Plants vs. Zombies: Game of the Year Edition** (Steam) and open ADDITION. It should appear in your library with a **Trainer** badge. If it doesn't, press **Rescan All**.
2. Open it and go to the **Trainer** tab.
   - If you have more than one version, pick yours under **Version**.
   - The Steam copy runs as `popcapgame1.exe`. The original PopCap release runs as `PlantsVsZombies.exe`.
3. Press **▶ Play**, or start the game however you like. The status changes from *Waiting for popcapgame1.exe* to **Trainer active**.
4. Start a level. Sun only exists while you're in a level, so cheats show an error on the main menu. That's normal.
5. Type a number next to **Set Sun** and press **Set**, or press **F1**. Then try **Unlimited Sun** (F2).

If the Sun counter changes, it works. Open the trainer file (Settings → *Open trainers folder*) and set `"verified": true`. If you get an error, or nothing changes, go to part 2.

> **"Access denied"?** The game is probably running as administrator. Run ADDITION as administrator too.

## Part 2: find Sun yourself

### Find the address

1. Be in a level and note your Sun. It starts at **50**.
2. Open the **Scanner** tab. The game's process is preselected (marked ★). Press **Attach**.
3. Leave the type on **i32**, set **Exact value**, type `50`, and press **First scan**. You'll get thousands of results, because lots of memory happens to hold 50.
4. Back in the game, collect a sun so the counter goes to 75. Type `75` and press **Next scan**.
5. Plant something to spend Sun, then scan for the new number. Repeat until only one to three results are left.
6. Press **Add** on a result. In **Your addresses**, change its value to `5000` and press Enter. If the game now shows 5000 Sun, you found it.

### Make it survive restarts

The address you found is on the heap. Next level or next launch, Sun will be somewhere else. What doesn't move is the chain of pointers that leads to it from the game's .exe, so find that next:

7. Press **Pointers** on your address. You'll get a list of candidate paths, best first.
8. Leave the level and start a new one. The game rebuilds the level, so Sun moves.
9. Find Sun again with a **New scan** (steps 3–5) and **Add** it. It shows up as a second entry.
10. In the pointer panel, choose your new entry under **Re-check against…** and press **Keep paths that still work**. Usually only a few survive, and those are the real ones. Repeat with another restart to be extra sure.
11. Press **Use** on the top path. The entry now shows something like `[["popcapgame1.exe"+0x355E0C]+0x868]+0x5578`.

### Save it

12. Press **Save as cheat**:
    - **Name:** Unlimited Sun
    - **Type:** *Toggle: keep it at a value*
    - **Keep it at:** 9990
    - **Hotkey:** F2
    - **Add to trainer:** the Plants vs. Zombies trainer. This saves your own copy, which replaces the bundled one.
13. Open the **Trainer** tab. Your cheat is there with a toggle and hotkey. Add a *Number box* version too if you want a **Set Sun**.

## Tips for other games

- **Health bars and speeds** are often `f32` (decimals). If an `i32` scan finds nothing, try `f32`. An exact `f32` scan for `100` also matches 100.3.
- **No number on screen?** First scan with **Unknown initial value**. Then lose some, scan **Decreased**. Wait, scan **Unchanged**. Gain some, scan **Increased**. Keep going until the list is short.
- **Money shown ×10 or ×100.** Some games store a different number than they show. PvZ's coins are one example: 5000 in memory is $50,000.
- **Code patches** (`"kind": "patch"`) need you to find the instruction that changes the value, which means using a debugger such as Cheat Engine's "find out what writes to this address". Then copy its bytes into a signature. The app runs these patches but can't make them yet.
- **Anti-cheat**: ADDITION won't attach while Easy Anti-Cheat, BattlEye, Vanguard and similar are running. Close those games first.
