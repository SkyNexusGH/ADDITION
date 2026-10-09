import { useEffect, useState } from "react";
import { dbq } from "../api/db";
import { api } from "../api/tauri";
import { useToast } from "../store/toast";
import Icon from "../components/Icon";
import { OnOff } from "./tabs/TrainerTab";
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
      <header className={styles.header}>
        <span className="label-caps">ADDITION</span>
        <h1 className="display-md">Settings</h1>
      </header>

      <Section title="Cover art" subtitle="Covers come from Steam by default. No key needed.">
        <Field
          label="SteamGridDB key (optional)"
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
        <div className={styles.pathRow}>
          <span className="label-caps">Trainers folder</span>
          <code className={styles.path}>{appData ? `${appData}/trainers` : "Not set up yet"}</code>
        </div>
        {appData && (
          <button className="ag-btn" onClick={() => api.openPath(`${appData}/trainers`)}>
            <Icon name="folder" />
            Open folder
          </button>
        )}
      </Section>

      <Section title="App">
        <div className={styles.toggleRow}>
          <span>Start ADDITION with Windows</span>
          <OnOff label="Start with Windows" on={s.startup} onChange={(on) => setS({ ...s, startup: on })} />
        </div>
      </Section>

      <button className={`ag-btn ag-btn--primary ${styles.save}`} onClick={onSave}>
        Save
      </button>

      <div className={styles.privacy}>
        <span className="label-caps">Privacy</span>
        <p>No telemetry, no account, no time limit. The only requests ADDITION makes online are for cover art.</p>
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
        <h2 className="title">{title}</h2>
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
