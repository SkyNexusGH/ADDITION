import { useEffect, useMemo, useRef, useState } from "react";
import { GameRow } from "../../api/db";
import {
  api,
  Cheat,
  formatPath,
  hex,
  isStatic,
  pathForHit,
  ProcessInfo,
  ScanFilter,
  Trainer,
  TrainerEntry,
  VALUE_TYPES,
  ValueType,
} from "../../api/tauri";
import { useScanner, WatchEntry } from "../../store/scanner";
import { useToast } from "../../store/toast";
import { Toggle } from "./TrainerTab";
import styles from "./Tabs.module.css";

type FilterKind = ScanFilter["kind"];

const FIRST_FILTERS: [FilterKind, string][] = [
  ["exact", "Exact value"],
  ["unknown", "Unknown initial value"],
];
const NEXT_FILTERS: [FilterKind, string][] = [
  ["exact", "Exact value"],
  ["decreased", "Decreased"],
  ["increased", "Increased"],
  ["changed", "Changed"],
  ["unchanged", "Unchanged"],
];

const basename = (p: string | null) => (p ? p.split(/[\\/]/).pop() ?? null : null);

export default function ScannerTab({ game }: { game: GameRow }) {
  const s = useScanner();
  const push = useToast((t) => t.push);
  const [error, setError] = useState<string | null>(null);

  // Keep watched values live.
  const watchRef = useRef(s.watch);
  watchRef.current = s.watch;
  useEffect(() => {
    if (!s.pid) return;
    const id = window.setInterval(async () => {
      const w = watchRef.current;
      if (!w.length) return;
      try {
        const vals = await api.scanRead(w.map((e) => ({ path: e.path, value_type: e.type })));
        useScanner.setState((st) => ({
          watch: st.watch.map((e) => {
            const i = w.findIndex((x) => x.key === e.key);
            return i >= 0 ? { ...e, value: vals[i] } : e;
          }),
        }));
      } catch {
        /* process gone; the picker shows it */
      }
    }, 500);
    return () => window.clearInterval(id);
  }, [s.pid]);

  return (
    <div className={styles.tab}>
      <ProcessPicker game={game} onError={setError} />
      {error && <code className={styles.error}>{error}</code>}

      {s.pid ? (
        <div className={styles.scanGrid}>
          <ScanPanel push={push} />
          <WatchPanel game={game} push={push} />
        </div>
      ) : (
        <div className={styles.panel}>
          <h3 className={styles.sectionTitle}>How making a cheat works</h3>
          <ol className={styles.steps}>
            <li>Start the game and pick its process above.</li>
            <li>Search for a number you can see, like your health (say it's 100).</li>
            <li>Change it in game (take a hit), then search for the new number. Repeat until only a few addresses are left.</li>
            <li>Add the address to your list and test it: change the value or freeze it.</li>
            <li>
              Click <strong>Pointers</strong> so the cheat still works after a restart, then <strong>Save as cheat</strong>.
              It shows up in the Trainer tab with a toggle and hotkey.
            </li>
          </ol>
        </div>
      )}
    </div>
  );
}

function ProcessPicker({ game, onError }: { game: GameRow; onError: (e: string | null) => void }) {
  const s = useScanner();
  const [procs, setProcs] = useState<ProcessInfo[]>([]);
  const [filter, setFilter] = useState("");
  const [pick, setPick] = useState<number | "">("");
  const [hints, setHints] = useState<string[]>([]);

  const refresh = async () => {
    try {
      setProcs(await api.listProcesses());
    } catch (e) {
      onError(String(e));
    }
  };

  useEffect(() => {
    refresh();
    // Suggest the game's own process: from its trainers or its exe path.
    api
      .trainersForGame(game.name, game.exe_path)
      .then((ts) => setHints([...ts.flatMap((t) => t.process), basename(game.exe_path) ?? ""].filter(Boolean)))
      .catch(() => setHints([basename(game.exe_path) ?? ""].filter(Boolean)));
  }, [game.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const suggested = useMemo(
    () => procs.find((p) => hints.some((h) => h.toLowerCase() === p.name.toLowerCase())),
    [procs, hints],
  );
  useEffect(() => {
    if (pick === "" && suggested) setPick(suggested.pid);
  }, [suggested]); // eslint-disable-line react-hooks/exhaustive-deps

  const shown = procs.filter((p) => !filter || p.name.toLowerCase().includes(filter.toLowerCase()));

  const attach = async () => {
    if (pick === "") return;
    onError(null);
    try {
      await api.scanOpen(pick);
      const name = procs.find((p) => p.pid === pick)?.name ?? null;
      s.set({ pid: pick, processName: name, summary: null, pointers: null, watch: [] });
    } catch (e) {
      onError(String(e));
    }
  };

  if (s.pid) {
    return (
      <div className={styles.statusBar + " " + styles.phase_attached}>
        <span className={styles.statusDot} />
        <div className={styles.statusText}>
          <strong>Scanning {s.processName}</strong>
          <span>Process {s.pid}</span>
        </div>
        <button
          className="btn"
          onClick={async () => {
            await api.scanClose();
            s.set({ pid: null, processName: null, summary: null, pointers: null });
          }}
        >
          Detach
        </button>
      </div>
    );
  }

  return (
    <div className={styles.panel}>
      <div className={styles.row}>
        <input placeholder="Filter processes…" value={filter} onChange={(e) => setFilter(e.target.value)} />
        <select className={styles.grow} value={pick} onChange={(e) => setPick(Number(e.target.value))}>
          <option value="">{procs.length ? "Choose the game's process" : "No processes found"}</option>
          {shown.map((p) => (
            <option key={p.pid} value={p.pid}>
              {p.name} ({p.pid}){p.pid === suggested?.pid ? " ★ this game" : ""}
            </option>
          ))}
        </select>
        <button className="btn" onClick={refresh}>
          Refresh
        </button>
        <button className="btn btn-primary" disabled={pick === ""} onClick={attach}>
          Attach
        </button>
      </div>
      {!suggested && hints.length > 0 && (
        <span className={styles.muted}>Looking for {hints.join(" or ")}. Start the game, then press Refresh.</span>
      )}
    </div>
  );
}

function ScanPanel({ push }: { push: (m: string, v?: any) => void }) {
  const s = useScanner();
  const first = !s.summary;
  const [kind, setKind] = useState<FilterKind>("exact");
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    if (first && !FIRST_FILTERS.some(([k]) => k === kind)) setKind("exact");
    if (!first && kind === "unknown") setKind("decreased");
  }, [first]); // eslint-disable-line react-hooks/exhaustive-deps

  const run = async () => {
    const filter: ScanFilter = kind === "exact" ? { kind, value } : { kind };
    setBusy(true);
    setProgress(0);
    const tick = window.setInterval(() => api.scanProgress().then(setProgress), 150);
    try {
      s.set({ summary: await api.scanRun(first, s.valueType, filter) });
      if (kind === "exact") setValue("");
    } catch (e) {
      push(String(e), "danger");
    } finally {
      window.clearInterval(tick);
      setBusy(false);
    }
  };

  const addHit = (i: number) => {
    const hit = s.summary!.hits[i];
    const entry: WatchEntry = {
      key: crypto.randomUUID(),
      label: `Value ${s.watch.length + 1}`,
      address: hit.address,
      path: pathForHit(hit),
      type: s.valueType,
      value: hit.value,
      frozen: false,
    };
    s.set({ watch: [...s.watch, entry] });
  };

  const filters = first ? FIRST_FILTERS : NEXT_FILTERS;
  const needsValue = kind === "exact";

  return (
    <div className={styles.panel}>
      <h3 className={styles.sectionTitle}>Find a value</h3>
      <div className={styles.row}>
        <select
          value={s.valueType}
          disabled={!first || busy}
          title="Most games store health, money and ammo as i32 (whole numbers) or f32 (decimals)"
          onChange={(e) => s.set({ valueType: e.target.value as ValueType })}
        >
          {VALUE_TYPES.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
        <select value={kind} disabled={busy} onChange={(e) => setKind(e.target.value as FilterKind)}>
          {filters.map(([k, label]) => (
            <option key={k} value={k}>
              {label}
            </option>
          ))}
        </select>
        {needsValue && (
          <input
            className={styles.grow}
            placeholder="Value"
            value={value}
            disabled={busy}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && value.trim() && run()}
          />
        )}
      </div>
      <div className={styles.row}>
        <button className="btn btn-primary" disabled={busy || (needsValue && !value.trim())} onClick={run}>
          {first ? "First scan" : "Next scan"}
        </button>
        {!first && (
          <button
            className="btn"
            disabled={busy}
            onClick={async () => {
              await api.scanReset();
              s.set({ summary: null });
            }}
          >
            New scan
          </button>
        )}
        {busy && (
          <button className="btn btn-ghost" onClick={() => api.scanCancel()}>
            Cancel
          </button>
        )}
        {s.summary && !busy && (
          <span className={styles.muted}>
            {s.summary.count.toLocaleString()} result{s.summary.count === 1 ? "" : "s"}
            {s.summary.count > s.summary.hits.length && s.summary.hits.length > 0
              ? ` (showing ${s.summary.hits.length})`
              : ""}
          </span>
        )}
      </div>
      {busy && (
        <div className={styles.progress}>
          <div style={{ width: `${progress / 10}%` }} />
        </div>
      )}

      {s.summary && s.summary.hits.length > 0 && (
        <div className={styles.tableWrap}>
          <table className={styles.table}>
            <thead>
              <tr>
                <th>Address</th>
                <th>Value</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {s.summary.hits.map((h, i) => (
                <tr key={h.address}>
                  <td className={styles.mono}>
                    {h.module_offset ? `${h.module_offset[0]}+${hex(h.module_offset[1])}` : hex(h.address)}
                  </td>
                  <td className={styles.mono}>{h.value}</td>
                  <td>
                    <button className={`btn ${styles.small}`} onClick={() => addHit(i)}>
                      Add
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {s.summary && s.summary.count > 50 && (
        <span className={styles.muted}>Too many. Change the value in game and run a next scan.</span>
      )}
      {s.summary && s.summary.count === 0 && (
        <span className={styles.muted}>
          Nothing matched. Try another type (f32 for decimals), or start over with "Unknown initial value".
        </span>
      )}
    </div>
  );
}

function WatchPanel({ game, push }: { game: GameRow; push: (m: string, v?: any) => void }) {
  const s = useScanner();
  const [saving, setSaving] = useState<WatchEntry | null>(null);
  const [finding, setFinding] = useState<string | null>(null);
  const [recheckWith, setRecheckWith] = useState<string>("");

  const writeValue = async (e: WatchEntry, raw: string) => {
    const v = Number(raw);
    if (raw.trim() === "" || !Number.isFinite(v)) return;
    try {
      await api.scanWrite(e.path, e.type, v);
      if (e.frozen) await api.scanFreeze(e.key, e.path, e.type, v);
    } catch (err) {
      push(String(err), "danger");
    }
  };

  const setFrozen = async (e: WatchEntry, on: boolean) => {
    try {
      await api.scanFreeze(e.key, e.path, e.type, on ? e.value : null);
      s.updateEntry(e.key, { frozen: on });
    } catch (err) {
      push(String(err), "danger");
    }
  };

  const findPointers = async (e: WatchEntry) => {
    setFinding(e.key);
    try {
      const paths = await api.scanFindPointers(e.address);
      s.set({ pointers: { key: e.key, paths } });
      if (!paths.length) push("No pointer paths found. Try again from a different point in the game.", "warning");
    } catch (err) {
      push(String(err), "danger");
    } finally {
      setFinding(null);
    }
  };

  const recheck = async () => {
    const other = s.watch.find((w) => w.key === recheckWith);
    if (!other || !s.pointers) return;
    try {
      const kept = await api.scanFilterPointers(
        s.pointers.paths.map((p) => p.path),
        other.address,
      );
      s.set({ pointers: { ...s.pointers, paths: kept } });
      push(`${kept.length} path${kept.length === 1 ? "" : "s"} still lead to the value`, kept.length ? "success" : "warning");
    } catch (err) {
      push(String(err), "danger");
    }
  };

  const pointerOwner = s.watch.find((w) => w.key === s.pointers?.key);

  return (
    <div className={styles.panel}>
      <h3 className={styles.sectionTitle}>Your addresses</h3>
      {s.watch.length === 0 ? (
        <span className={styles.muted}>Add results from the search to test them here.</span>
      ) : (
        <div className={styles.tableWrap}>
          <table className={styles.table}>
            <thead>
              <tr>
                <th>Name</th>
                <th>Where</th>
                <th>Value</th>
                <th title="Freeze">❄</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {s.watch.map((e) => (
                <tr key={e.key}>
                  <td>
                    <input
                      value={e.label}
                      style={{ width: 110 }}
                      onChange={(ev) => s.updateEntry(e.key, { label: ev.target.value })}
                    />
                  </td>
                  <td className={styles.mono} title={isStatic(e.path) ? "Static or pointer path" : "Heap address: changes when the game restarts"}>
                    {formatPath(e.path)} <span className={styles.muted}>{e.type}</span>
                  </td>
                  <td>
                    <input
                      key={e.frozen ? "f" : String(e.value)}
                      defaultValue={e.value ?? ""}
                      style={{ width: 90 }}
                      placeholder="??"
                      onKeyDown={(ev) => ev.key === "Enter" && writeValue(e, (ev.target as HTMLInputElement).value)}
                      onBlur={(ev) => ev.target.value !== String(e.value ?? "") && writeValue(e, ev.target.value)}
                    />
                  </td>
                  <td>
                    <Toggle checked={e.frozen} disabled={e.value == null} onChange={(on) => setFrozen(e, on)} />
                  </td>
                  <td>
                    <div className={styles.actions}>
                      <button
                        className={`btn ${styles.small}`}
                        disabled={finding !== null}
                        title="Find a path that still works after the game restarts"
                        onClick={() => findPointers(e)}
                      >
                        {finding === e.key ? "Searching…" : "Pointers"}
                      </button>
                      <button className={`btn btn-primary ${styles.small}`} onClick={() => setSaving(e)}>
                        Save as cheat
                      </button>
                      <button
                        className={`btn btn-ghost ${styles.small}`}
                        onClick={async () => {
                          if (e.frozen) await api.scanFreeze(e.key, e.path, e.type, null).catch(() => {});
                          s.removeEntry(e.key);
                        }}
                      >
                        ✕
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {s.pointers && pointerOwner && (
        <>
          <h3 className={styles.sectionTitle}>Pointer paths to “{pointerOwner.label}”</h3>
          <span className={styles.muted}>
            {s.pointers.paths.length} found, best first. Pick one with <strong>Use</strong>. To be sure it survives
            a restart: restart the game, attach again, find the value again, add it to the list, then re-check
            below.
          </span>
          <div className={styles.row}>
            <select value={recheckWith} onChange={(e) => setRecheckWith(e.target.value)}>
              <option value="">Re-check against…</option>
              {s.watch
                .filter((w) => w.key !== pointerOwner.key)
                .map((w) => (
                  <option key={w.key} value={w.key}>
                    {w.label} ({hex(w.address)})
                  </option>
                ))}
            </select>
            <button className="btn" disabled={!recheckWith} onClick={recheck}>
              Keep paths that still work
            </button>
          </div>
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <tbody>
                {s.pointers.paths.slice(0, 100).map((p) => (
                  <tr key={p.display}>
                    <td className={styles.mono}>{p.display}</td>
                    <td>
                      <button
                        className={`btn ${styles.small}`}
                        onClick={() => {
                          s.updateEntry(pointerOwner.key, { path: p.path });
                          push("Path set. Save as cheat when you're happy with it.", "success");
                        }}
                      >
                        Use
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {saving && <SaveCheatDialog entry={saving} game={game} onClose={() => setSaving(null)} />}
    </div>
  );
}

const slug = (s: string) =>
  s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "") || "cheat";

function SaveCheatDialog({ entry, game, onClose }: { entry: WatchEntry; game: GameRow; onClose: () => void }) {
  const processName = useScanner((s) => s.processName);
  const push = useToast((t) => t.push);
  const [trainers, setTrainers] = useState<TrainerEntry[]>([]);
  const [target, setTarget] = useState<string>("new");
  const [name, setName] = useState(entry.label.startsWith("Value ") ? "" : entry.label);
  const [kind, setKind] = useState<"freeze" | "set">("freeze");
  const [value, setValue] = useState(String(entry.value ?? ""));
  const [min, setMin] = useState("0");
  const [max, setMax] = useState("");
  const [hotkey, setHotkey] = useState("");

  useEffect(() => {
    api.trainersForGame(game.name, game.exe_path).then((ts) => {
      setTrainers(ts);
      if (ts.length) setTarget(ts[0].id);
    });
  }, [game.name, game.exe_path]);

  const save = async () => {
    const v = Number(value);
    if (!name.trim() || !Number.isFinite(v)) return;
    let trainer: Trainer;
    const existing = trainers.find((t) => t.id === target);
    if (existing) {
      const { origin: _o, path: _p, ...rest } = existing;
      trainer = rest;
    } else {
      trainer = {
        id: `${slug(game.name)}-mine`,
        game: game.name,
        process: processName ? [processName] : [],
        author: "me",
        verified: true,
        cheats: [],
      };
    }
    const ids = new Set(trainer.cheats.map((c) => c.id));
    let id = slug(name);
    for (let n = 2; ids.has(id); n++) id = `${slug(name)}-${n}`;
    const base = { id, name: name.trim(), hotkey: hotkey.trim() || null, target: entry.path, type: entry.type };
    const cheat: Cheat =
      kind === "freeze"
        ? { ...base, kind: "freeze", value: v }
        : {
            ...base,
            kind: "set",
            default: v,
            ...(min.trim() !== "" ? { min: Number(min) } : {}),
            ...(max.trim() !== "" ? { max: Number(max) } : {}),
          };
    try {
      const path = await api.saveTrainer({ ...trainer, cheats: [...trainer.cheats, cheat] });
      push(`Saved "${cheat.name}" to ${path}`, "success");
      onClose();
    } catch (e) {
      push(String(e), "danger");
    }
  };

  return (
    <div className={styles.dialog} onClick={onClose}>
      <div className={styles.dialogBody} onClick={(e) => e.stopPropagation()}>
        <h3>Save as cheat</h3>
        {!isStatic(entry.path) && (
          <div className={styles.disclaimer}>
            This is a heap address, so it will move when the game restarts. Use <strong>Pointers</strong> first to get
            a path that lasts.
          </div>
        )}
        <label className={styles.field}>
          <span>Name</span>
          <input autoFocus value={name} placeholder="Unlimited Health" onChange={(e) => setName(e.target.value)} />
        </label>
        <label className={styles.field}>
          <span>Type of cheat</span>
          <select value={kind} onChange={(e) => setKind(e.target.value as "freeze" | "set")}>
            <option value="freeze">Toggle: keep it at a value (Unlimited …)</option>
            <option value="set">Number box: set it to anything (Set …)</option>
          </select>
        </label>
        <label className={styles.field}>
          <span>{kind === "freeze" ? "Keep it at" : "Default value"}</span>
          <input value={value} onChange={(e) => setValue(e.target.value)} />
        </label>
        {kind === "set" && (
          <div className={styles.row}>
            <label className={styles.field}>
              <span>Min</span>
              <input value={min} onChange={(e) => setMin(e.target.value)} style={{ width: 110 }} />
            </label>
            <label className={styles.field}>
              <span>Max</span>
              <input value={max} placeholder="none" onChange={(e) => setMax(e.target.value)} style={{ width: 110 }} />
            </label>
          </div>
        )}
        <label className={styles.field}>
          <span>Hotkey (optional)</span>
          <input value={hotkey} placeholder="F1, Ctrl+F2, …" onChange={(e) => setHotkey(e.target.value)} />
        </label>
        <label className={styles.field}>
          <span>Add to trainer</span>
          <select value={target} onChange={(e) => setTarget(e.target.value)}>
            {trainers.map((t) => (
              <option key={t.id} value={t.id}>
                {t.game} {t.game_version ? `(${t.game_version})` : ""} {t.origin === "bundled" ? "· saves your own copy" : ""}
              </option>
            ))}
            <option value="new">New trainer for {game.name}</option>
          </select>
        </label>
        <div className={styles.row}>
          <button className="btn btn-primary" disabled={!name.trim() || value.trim() === ""} onClick={save}>
            Save
          </button>
          <button className="btn btn-ghost" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
