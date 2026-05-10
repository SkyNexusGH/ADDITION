import { useEffect, useState } from "react";
import { dbq } from "../api/db";
import { api } from "../api/tauri";
import { nexusLogin, nexusLogout, nexusStatus } from "../api/nexusSso";
import { useToast } from "../store/toast";
import styles from "./SettingsPage.module.css";

interface SettingsState {
  curseforge: string;
  steamgriddb: string;
  download_path: string;
  startup: boolean;
}

const DEFAULTS: SettingsState = {
  curseforge: "",
  steamgriddb: "",
  download_path: "",
  startup: false,
};

const HAS_BUNDLED_CF = !!(import.meta.env.VITE_CURSEFORGE_API_KEY ?? "");

export default function SettingsPage() {
  const [s, setS] = useState<SettingsState>(DEFAULTS);
  const [appData, setAppData] = useState<string>("");
  const [advanced, setAdvanced] = useState(false);
  const [nexus, setNexus] = useState<{ signedIn: boolean; signedInAt: string | null }>({
    signedIn: false,
    signedInAt: null,
  });
  const [signingIn, setSigningIn] = useState(false);
  const [signInMsg, setSignInMsg] = useState<string>("");
  const push = useToast((p) => p.push);

  useEffect(() => {
    (async () => {
      setS({
        curseforge: (await dbq.getSetting("api_curseforge")) ?? "",
        steamgriddb: (await dbq.getSetting("api_steamgriddb")) ?? "",
        download_path: (await dbq.getSetting("download_path")) ?? "",
        startup: (await dbq.getSetting("startup")) === "1",
      });
      setNexus(await nexusStatus());
      try {
        setAppData(await api.appDataDir());
      } catch {
        setAppData("");
      }
    })();
  }, []);

  const onSave = async () => {
    await dbq.setSetting("api_curseforge", s.curseforge);
    await dbq.setSetting("api_steamgriddb", s.steamgriddb);
    await dbq.setSetting("download_path", s.download_path);
    await dbq.setSetting("startup", s.startup ? "1" : "0");
    push("Settings saved", "success");
  };

  const onSignIn = async () => {
    setSigningIn(true);
    setSignInMsg("");
    try {
      await nexusLogin((m) => setSignInMsg(m));
      setNexus(await nexusStatus());
      push("Signed in to Nexus Mods", "success");
    } catch (e: any) {
      push(`Nexus sign-in failed: ${e?.message ?? e}`, "danger");
    } finally {
      setSigningIn(false);
      setSignInMsg("");
    }
  };

  const onSignOut = async () => {
    await nexusLogout();
    setNexus({ signedIn: false, signedInAt: null });
    push("Signed out of Nexus Mods", "info");
  };

  return (
    <div className={styles.page}>
      <h1 className={styles.title}>Settings</h1>

      <Section
        title="Mod sources"
        subtitle="One-click for Nexus, automatic for CurseForge — no copy-pasting keys."
      >
        <div className={styles.sourceRow}>
          <div className={styles.sourceMeta}>
            <strong>Nexus Mods</strong>
            <span className={styles.muted}>
              {nexus.signedIn
                ? `Signed in${nexus.signedInAt ? ` · ${new Date(nexus.signedInAt).toLocaleString()}` : ""}`
                : "Not signed in"}
            </span>
          </div>
          {nexus.signedIn ? (
            <button className="btn" onClick={onSignOut}>Sign out</button>
          ) : (
            <button
              className="btn btn-primary"
              onClick={onSignIn}
              disabled={signingIn}
            >
              {signingIn ? signInMsg || "Waiting for browser…" : "Sign in with Nexus"}
            </button>
          )}
        </div>

        <div className={styles.sourceRow}>
          <div className={styles.sourceMeta}>
            <strong>CurseForge</strong>
            <span className={styles.muted}>
              {HAS_BUNDLED_CF
                ? "Connected via the bundled application key — no setup required."
                : "No bundled key in this build. Add VITE_CURSEFORGE_API_KEY at build time, or paste a personal key under Advanced."}
            </span>
          </div>
          <span className={`badge ${HAS_BUNDLED_CF ? styles.ok : styles.warn}`}>
            {HAS_BUNDLED_CF ? "Ready" : "Manual key required"}
          </span>
        </div>

        <button
          className="btn btn-ghost"
          onClick={() => setAdvanced((v) => !v)}
        >
          {advanced ? "Hide advanced" : "Show advanced overrides"}
        </button>

        {advanced && (
          <>
            <Field
              label="CurseForge API Key (override)"
              help="Optional. Use a personal key if the bundled one rate-limits you."
            >
              <input
                type="password"
                value={s.curseforge}
                onChange={(e) => setS({ ...s, curseforge: e.target.value })}
                placeholder="$2a$10$..."
              />
            </Field>
            <Field
              label="SteamGridDB API Key (cover art override)"
              help="Optional. ADDITION fetches covers from Steam's CDN by default — no key needed. This is only for higher-quality / community uploads."
            >
              <input
                type="password"
                value={s.steamgriddb}
                onChange={(e) => setS({ ...s, steamgriddb: e.target.value })}
              />
            </Field>
          </>
        )}
      </Section>

      <Section title="Storage" subtitle="Where ADDITION keeps mods and backups.">
        <Field label="Download path" help="Leave blank to use the default (per-user app data).">
          <input
            type="text"
            value={s.download_path}
            onChange={(e) => setS({ ...s, download_path: e.target.value })}
            placeholder={appData || "%APPDATA%/io.addition.app/downloads"}
          />
        </Field>
        <div className={styles.muted}>
          <strong>App data:</strong> {appData || "(not yet initialized)"}
        </div>
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
        <strong>Privacy.</strong> ADDITION sends zero telemetry. Outbound requests only hit
        the mod sources you've connected — Nexus' SSO server during sign-in, and the mod
        provider APIs when you browse mods.
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
