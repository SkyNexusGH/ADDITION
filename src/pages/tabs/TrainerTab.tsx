import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { GameRow } from "../../api/db";
import { api, Cheat, CheatState, HostStatus, Launcher, TrainerEntry } from "../../api/tauri";
import { useTrainer } from "../../store/trainer";
import { useToast } from "../../store/toast";
import Icon from "../../components/Icon";
import styles from "./Tabs.module.css";

export default function TrainerTab({ game }: { game: GameRow }) {
  const [trainers, setTrainers] = useState<TrainerEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { trainer, status, open, close } = useTrainer();
  const push = useToast((s) => s.push);

  useEffect(() => {
    setTrainers(null);
    api
      .trainersForGame(game.name, game.exe_path)
      .then(setTrainers)
      .catch((e) => setError(String(e)));
  }, [game.name, game.exe_path]);

  // Open this game's trainer unless another game's trainer is running.
  useEffect(() => {
    if (!trainers?.length) return;
    const mine = trainers.find((t) => t.id === trainer?.id);
    if (mine) {
      if (JSON.stringify(mine) !== JSON.stringify(trainer)) open(mine).catch((e) => setError(String(e)));
    } else if (!trainer) {
      open(trainers[0]).catch((e) => setError(String(e)));
    }
  }, [trainers]); // eslint-disable-line react-hooks/exhaustive-deps

  if (error) {
    return (
      <div className={styles.empty}>
        <h3 className="title">Couldn't load trainers</h3>
        <code className={styles.error}>{error}</code>
      </div>
    );
  }
  if (trainers === null) return <div className={styles.muted}>Loading trainers</div>;

  if (trainers.length === 0) {
    return (
      <div className={styles.empty}>
        <h3 className="title">No trainer for {game.name} yet</h3>
        <p>
          Make one yourself in the <Link to="../scanner">Scanner</Link> tab: start the game, search
          for a number you can see (health, money, ammo), change it in game, search again, then save
          what you find as a cheat. <code>docs/first-trainer.md</code> walks through it.
        </p>
      </div>
    );
  }

  const mine = trainers.find((t) => t.id === trainer?.id);
  if (!mine) {
    return (
      <div className={styles.empty}>
        <h3 className="title">Another trainer is running</h3>
        <p>
          {trainer?.game} is attached right now. Only one trainer runs at a time.
        </p>
        <button
          className="ag-btn ag-btn--primary"
          onClick={async () => {
            await close();
            await open(trainers[0]);
            push(`Switched to ${trainers[0].game}`, "info");
          }}
        >
          Switch to {game.name}
        </button>
      </div>
    );
  }

  const verifiedWord = mine.verified ? "Verified" : "Unverified";
  return (
    <div className={styles.tab}>
      <StatusBar trainer={mine} status={status} game={game} />

      <div className={styles.trainerMeta}>
        <span className="label-caps">Cheats</span>
        <span className={`ag-chip ${mine.verified ? "" : "ag-chip--new"}`}>{verifiedWord}</span>
        {mine.game_version && <span className="ag-tag">{mine.game_version}</span>}
        {mine.origin === "user" && <span className="ag-tag">Yours</span>}
        {trainers.length > 1 && (
          <select
            className={styles.versionPicker}
            aria-label="Trainer version"
            value={mine.id}
            onChange={(e) => open(trainers.find((t) => t.id === e.target.value)!)}
          >
            {trainers.map((t) => (
              <option key={t.id} value={t.id}>
                {t.game_version || t.id}
                {t.origin === "user" ? " (yours)" : ""}
              </option>
            ))}
          </select>
        )}
      </div>

      {!mine.verified && mine.notes && <p className={styles.note}>{mine.notes}</p>}

      {status?.hotkey_errors.length ? (
        <p className={styles.note}>
          <span className={styles.errorWord}>Error</span> Some hotkeys couldn't be registered:{" "}
          {status.hotkey_errors.join("; ")}
        </p>
      ) : null}

      {mine.cheats.length === 0 && (
        <p className={styles.note}>
          No cheats yet. Start the game, find a value in the <Link to="../scanner">Scanner</Link> tab, and use{" "}
          <strong>Save as cheat</strong> to add it here.
        </p>
      )}

      <div className={styles.cheatList}>
        {mine.cheats.map((c) => (
          <CheatRow
            key={c.id}
            cheat={c}
            state={status?.cheats.find((s) => s.id === c.id)}
            attached={status?.phase === "attached"}
          />
        ))}
      </div>

      <p className={styles.footnote}>
        Single-player only. ADDITION won't attach while anti-cheat is running.
        {mine.origin === "user" && mine.path ? ` Trainer file: ${mine.path}` : ""}
      </p>
    </div>
  );
}

function StatusBar({ trainer, status, game }: { trainer: TrainerEntry; status: HostStatus | null; game: GameRow }) {
  const push = useToast((s) => s.push);
  const phase = status?.phase ?? "idle";
  const chip = {
    idle: { word: "Off", cls: "" },
    waiting: { word: "Waiting", cls: "" },
    attached: { word: "Live", cls: "ag-chip--live" },
    blocked: { word: "Blocked", cls: "" },
    error: { word: "Error", cls: "" },
  }[phase];
  const title = {
    idle: "Trainer off",
    waiting: `Waiting for ${trainer.process[0]}`,
    attached: "Trainer on",
    blocked: "Anti-cheat found",
    error: "Couldn't attach",
  }[phase];
  const detail =
    status?.message ??
    (phase === "waiting"
      ? "Start the game. The trainer attaches by itself."
      : phase === "attached"
        ? `Attached to process ${status?.pid}. Hotkeys work while the game has focus.`
        : "");

  return (
    <div className={`${styles.statusBar} ${phase === "attached" ? styles.statusLive : ""}`}>
      <span className={`ag-chip ${chip.cls}`}>{chip.word}</span>
      <div className={styles.statusText}>
        <strong>{title}</strong>
        {detail && <span className={phase === "blocked" || phase === "error" ? styles.errorText : ""}>{detail}</span>}
      </div>
      {phase !== "attached" && (
        <button
          className="ag-btn ag-btn--ember"
          onClick={() =>
            api
              .launchGame(game.launcher as Launcher, game.app_id, game.exe_path)
              .then(() => push(`Starting ${game.name}`, "info"))
              .catch((e) => push(`Couldn't start the game: ${e}`, "danger"))
          }
        >
          <Icon name="play" />
          Play
        </button>
      )}
    </div>
  );
}

function CheatRow({ cheat, state, attached }: { cheat: Cheat; state?: CheatState; attached: boolean }) {
  const apply = useTrainer((s) => s.apply);
  const push = useToast((s) => s.push);
  const active = !!state?.active;
  const run = (p: Promise<HostStatus>) => apply(p).catch((e) => push(String(e), "danger"));

  return (
    <div className={`${styles.cheatRow} ${active ? styles.on : ""} ${attached ? "" : styles.disabled}`}>
      <div className={styles.cheatInfo}>
        <div className={styles.cheatName}>
          <span className="subtitle">{cheat.name}</span>
          {cheat.hotkey && <kbd className="ag-tag ag-tag--mono">{cheat.hotkey}</kbd>}
        </div>
        {cheat.description && <div className={styles.cheatDesc}>{cheat.description}</div>}
        {state?.error && (
          <div className={styles.cheatError}>
            <span className={styles.errorWord}>Error</span> {state.error}
          </div>
        )}
      </div>
      <div className={styles.cheatControls}>
        {cheat.kind === "set" ? (
          <SetControl cheat={cheat} state={state} disabled={!attached} run={run} />
        ) : (
          <>
            {cheat.kind === "freeze" && state?.value != null && (
              <span className="ag-well" title="Current value in game">
                {fmt(state.value)}
              </span>
            )}
            <OnOff
              label={cheat.name}
              on={active}
              disabled={!attached}
              onChange={(on) => run(on ? api.cheatEnable(cheat.id) : api.cheatDisable(cheat.id))}
            />
          </>
        )}
      </div>
    </div>
  );
}

function SetControl({
  cheat,
  state,
  disabled,
  run,
}: {
  cheat: Extract<Cheat, { kind: "set" }>;
  state?: CheatState;
  disabled: boolean;
  run: (p: Promise<HostStatus>) => void;
}) {
  const [value, setValue] = useState<string>(String(cheat.default ?? cheat.min ?? 0));
  const num = Number(value);
  const valid = value.trim() !== "" && Number.isFinite(num);
  const slider = cheat.min != null && cheat.max != null && cheat.max - cheat.min <= 100000;
  const step = cheat.type === "f32" || cheat.type === "f64" ? "any" : 1;

  return (
    <>
      <span className="ag-well" title="Current value in game">
        {state?.value != null ? fmt(state.value) : "–"}
      </span>
      {slider && (
        <input
          type="range"
          aria-label={`${cheat.name} slider`}
          min={cheat.min}
          max={cheat.max}
          step={step === "any" ? (cheat.max! - cheat.min!) / 100 : 1}
          value={valid ? num : cheat.min}
          disabled={disabled}
          onChange={(e) => setValue(e.target.value)}
        />
      )}
      <input
        type="number"
        aria-label={cheat.name}
        value={value}
        min={cheat.min}
        max={cheat.max}
        step={step}
        disabled={disabled}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && valid && run(api.cheatSet(cheat.id, num))}
      />
      <button className="ag-btn ag-btn--sm" disabled={disabled || !valid} onClick={() => run(api.cheatSet(cheat.id, num))}>
        Set
      </button>
      <button
        className={`ag-icon-btn ${state?.active ? "is-active" : ""}`}
        aria-label={state?.active ? `Unlock ${cheat.name}` : `Lock ${cheat.name} at this value`}
        aria-pressed={!!state?.active}
        title={state?.active ? "Locked. Click to unlock" : "Lock at this value"}
        disabled={disabled || !valid}
        onClick={() => run(state?.active ? api.cheatDisable(cheat.id) : api.cheatEnable(cheat.id, num))}
      >
        <Icon name="lock" />
      </button>
    </>
  );
}

/** Afterglow Off/On segmented switch: both states are labelled. */
export function OnOff({
  on,
  disabled,
  onChange,
  label,
}: {
  on: boolean;
  disabled?: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <div className="ag-seg" role="group" aria-label={label}>
      <button className={!on ? "is-on" : ""} aria-pressed={!on} disabled={disabled} onClick={() => on && onChange(false)}>
        Off
      </button>
      <button className={on ? "is-on" : ""} aria-pressed={on} disabled={disabled} onClick={() => !on && onChange(true)}>
        On
      </button>
    </div>
  );
}

const fmt = (v: number) => (Number.isInteger(v) ? String(v) : v.toFixed(2));
