import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { GameRow } from "../../api/db";
import { api, Cheat, CheatState, HostStatus, Launcher, TrainerEntry } from "../../api/tauri";
import { useTrainer } from "../../store/trainer";
import { useToast } from "../../store/toast";
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
        <h3>Couldn't load trainers</h3>
        <code className={styles.error}>{error}</code>
      </div>
    );
  }
  if (trainers === null) return <div className={styles.muted}>Loading trainers…</div>;

  if (trainers.length === 0) {
    return (
      <div className={styles.empty}>
        <h3>No trainer for {game.name} yet</h3>
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
        <h3>Another trainer is running</h3>
        <p>
          {trainer?.game} is attached right now. Only one trainer runs at a time.
        </p>
        <button
          className="btn btn-primary"
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

  return (
    <div className={styles.tab}>
      {trainers.length > 1 && (
        <div className={styles.trainerPicker}>
          <span className={styles.muted}>Version:</span>
          <select value={mine.id} onChange={(e) => open(trainers.find((t) => t.id === e.target.value)!)}>
            {trainers.map((t) => (
              <option key={t.id} value={t.id}>
                {t.game_version || t.id}
                {t.origin === "user" ? " (yours)" : ""}
              </option>
            ))}
          </select>
        </div>
      )}

      <StatusBar trainer={mine} status={status} game={game} />

      {!mine.verified && (
        <div className={styles.disclaimer}>
          <strong>Unverified.</strong> {mine.notes || "These cheats haven't been confirmed on your game version yet."}
        </div>
      )}

      {status?.hotkey_errors.length ? (
        <div className={styles.disclaimer}>
          Some hotkeys couldn't be registered: {status.hotkey_errors.join("; ")}
        </div>
      ) : null}

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

      <p className={styles.muted}>
        Single-player only. ADDITION refuses to attach when it sees anti-cheat. {mine.origin === "user" && mine.path ? `Trainer file: ${mine.path}` : ""}
      </p>
    </div>
  );
}

function StatusBar({ trainer, status, game }: { trainer: TrainerEntry; status: HostStatus | null; game: GameRow }) {
  const push = useToast((s) => s.push);
  const phase = status?.phase ?? "idle";
  const title = {
    idle: "Trainer off",
    waiting: `Waiting for ${trainer.process[0]}`,
    attached: "Trainer active",
    blocked: "Blocked",
    error: "Couldn't attach",
  }[phase];
  const detail =
    status?.message ??
    (phase === "waiting"
      ? "Start the game and the trainer attaches by itself."
      : phase === "attached"
        ? `Attached to process ${status?.pid}. Hotkeys work while the game has focus.`
        : "");

  return (
    <div className={`${styles.statusBar} ${styles[`phase_${phase}`]}`}>
      <span className={styles.statusDot} />
      <div className={styles.statusText}>
        <strong>{title}</strong>
        {detail && <span>{detail}</span>}
      </div>
      {phase !== "attached" && (
        <button
          className="btn btn-primary"
          onClick={() =>
            api
              .launchGame(game.launcher as Launcher, game.app_id, game.exe_path)
              .then(() => push(`Launching ${game.name}…`, "info"))
              .catch((e) => push(`Launch failed: ${e}`, "danger"))
          }
        >
          ▶ Play
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
          {cheat.name}
          {cheat.hotkey && <span className={styles.key}>{cheat.hotkey}</span>}
        </div>
        {cheat.description && <div className={styles.cheatDesc}>{cheat.description}</div>}
        {state?.error && <div className={styles.cheatError}>{state.error}</div>}
      </div>
      <div className={styles.cheatControls}>
        {cheat.kind === "set" ? (
          <SetControl cheat={cheat} state={state} disabled={!attached} run={run} />
        ) : (
          <>
            {cheat.kind === "freeze" && state?.value != null && (
              <span className={styles.liveValue}>{fmt(state.value)}</span>
            )}
            <Toggle
              checked={active}
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
      <span className={styles.liveValue} title="Current value in game">
        {state?.value != null ? fmt(state.value) : "–"}
      </span>
      {slider && (
        <input
          type="range"
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
        value={value}
        min={cheat.min}
        max={cheat.max}
        step={step}
        disabled={disabled}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && valid && run(api.cheatSet(cheat.id, num))}
      />
      <button className="btn" disabled={disabled || !valid} onClick={() => run(api.cheatSet(cheat.id, num))}>
        Set
      </button>
      <Toggle
        title="Lock at this value"
        checked={!!state?.active}
        disabled={disabled || !valid}
        onChange={(on) => run(on ? api.cheatEnable(cheat.id, num) : api.cheatDisable(cheat.id))}
      />
    </>
  );
}

export function Toggle({
  checked,
  disabled,
  onChange,
  title,
}: {
  checked: boolean;
  disabled?: boolean;
  onChange: (v: boolean) => void;
  title?: string;
}) {
  return (
    <label className={styles.toggle} title={title}>
      <input type="checkbox" checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} />
      <span />
    </label>
  );
}

const fmt = (v: number) => (Number.isInteger(v) ? String(v) : v.toFixed(2));
