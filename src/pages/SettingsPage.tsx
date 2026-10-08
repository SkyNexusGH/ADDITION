import { useEffect, useState } from "react";
import { dbq } from "../api/db";
import { api } from "../api/tauri";
import { useToast } from "../store/toast";
import styles from "./SettingsPage.module.css";

interface SettingsState {
  steamgriddb: string;
  startup: boolean;
}

const DEFAULTS: SettingsState = {
  steamgriddb: "",
  startup: false,
};

export default function SettingsPage() {
  const [s, setS] = useState<SettingsState>(DEFAULTS);
  const [appData, setAppData] = useState<string>("");
  const push = useToast((p) => p.push);

  useEffect(() => {
    (async () => {
      setS({
        steamgriddb: (await dbq.getSetting("api_steamgriddb")) ?? "",
        startup: (await dbq.getSetting("startup")) === "1",
      });
      try {
        setAppData(await api.appDataDir());
      } catch {
        setAppData("");
      }
    })();
  }, []);

  const onSave = async () => {
    await dbq.setSetting("api_steamgriddb", s.steamgriddb);
    await dbq.setSetting("startup", s.startup ? "1" : "0");
    push("Settings saved", "success");
  };

  return (
    <div className={styles.page}>
      <h1 className={styles.title}>Settings</h1>

      <Section title="Cover art" subtitle="Covers come from Steam's CDN by default. No key needed.">
        <Field
          label="SteamGridDB API Key (optional)"
          help="Only needed for higher-quality community covers."
        >
          <input
            type="password"
            value={s.steamgriddb}
            onChange={(e) => setS({ ...s, steamgriddb: e.target.value })}
          />
        </Field>
      </Section>

      <Section title="Storage" subtitle="Your trainers live here as plain JSON files.">
        <div className={styles.muted}>
          <strong>Trainers folder:</strong> {appData ? `${appData}/trainers` : "(not yet initialized)"}
        </div>
        {appData && (
          <button className="btn" onClick={() => api.openPath(`${appData}/trainers`)}>
            Open trainers folder
          </button>
        )}
      </Section>

      <Section title="App" subtitle="Behaviour preferences.">
        <label className={styles.toggleRow}>
          <input
            type="checkbox"
            checked={s.startup}
            onChange={(e) => setS({ ...s, startup: e.target.checked })}
          />
          <span>Launch ADDITION on system startup</span>
        </label>
        <div className={styles.muted}>
          <strong>Theme:</strong> Dark · MVP only
        </div>
      </Section>

      <button className="btn btn-primary" onClick={onSave}>Save settings</button>

      <div className={styles.privacy}>
        <strong>Privacy.</strong> ADDITION sends zero telemetry and has no accounts or time
        limits. The only outbound requests are for cover art.
      </div>
    </div>
  );
}

function Section({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: string;
  children: React.ReactNode;
}) {
  return (
    <section className={styles.section}>
      <header>
        <h2>{title}</h2>
        {subtitle && <p>{subtitle}</p>}
      </header>
      <div className={styles.body}>{children}</div>
    </section>
  );
}

function Field({
  label,
  help,
  children,
}: {
  label: string;
  help?: string;
  children: React.ReactNode;
}) {
  return (
    <label className={styles.field}>
      <span>{label}</span>
      {children}
      {help && <em>{help}</em>}
    </label>
  );
}
