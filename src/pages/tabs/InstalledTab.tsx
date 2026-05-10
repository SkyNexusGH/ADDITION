import { useEffect, useState } from "react";
import { GameRow, InstalledModRow, dbq } from "../../api/db";
import { api, BackupEntry } from "../../api/tauri";
import { useToast } from "../../store/toast";
import styles from "./Tabs.module.css";

export default function InstalledTab({ game }: { game: GameRow }) {
  const [mods, setMods] = useState<InstalledModRow[]>([]);
  const [backups, setBackups] = useState<BackupEntry[]>([]);
  const push = useToast((s) => s.push);

  const refresh = async () => {
    setMods(await dbq.listInstalledMods(game.id));
    setBackups(await api.listBackups(game.id));
  };

  useEffect(() => {
    refresh();
  }, [game.id]);

  const onUninstall = async (mod: InstalledModRow) => {
    if (!mod.backup_id) {
      push("No backup recorded — cannot safely uninstall.", "warning");
      return;
    }
    try {
      await api.uninstallMod(game.id, game.install_path, mod.backup_id);
      await dbq.deleteInstalledMod(mod.id);
      await refresh();
      push(`Uninstalled ${mod.name}`, "success");
    } catch (e: any) {
      push(`Uninstall failed: ${e?.toString?.()}`, "danger");
    }
  };

  const onToggle = async (mod: InstalledModRow) => {
    await dbq.setModEnabled(mod.id, !mod.enabled);
    await refresh();
  };

  const onRestore = async (backup: BackupEntry) => {
    if (!confirm(`Restore backup from ${backup.created_at}? This will overwrite current files.`))
      return;
    try {
      await api.restoreBackup(game.id, game.install_path, backup.id);
      push("Backup restored", "success");
      await refresh();
    } catch (e: any) {
      push(`Restore failed: ${e?.toString?.()}`, "danger");
    }
  };

  return (
    <div className={styles.tab}>
      <section>
        <h3 className={styles.sectionTitle}>Installed Mods ({mods.length})</h3>
        {mods.length === 0 ? (
          <div className={styles.empty}>
            <p>No mods installed yet.</p>
          </div>
        ) : (
          <div className={styles.installedList}>
            {mods.map((m) => (
              <div key={m.id} className={styles.installedRow}>
                <label className={styles.toggle}>
                  <input
                    type="checkbox"
                    checked={!!m.enabled}
                    onChange={() => onToggle(m)}
                  />
                  <span />
                </label>
                <div className={styles.installedName}>
                  <strong>{m.name}</strong>
                  <span className={styles.muted}>
                    v{m.version} · {(m.size_bytes / (1024 * 1024)).toFixed(1)} MB · {m.source}
                  </span>
                </div>
                <button className="btn" onClick={() => onUninstall(m)}>Uninstall</button>
              </div>
            ))}
          </div>
        )}
      </section>

      <section>
        <h3 className={styles.sectionTitle}>Backups ({backups.length})</h3>
        {backups.length === 0 ? (
          <div className={styles.empty}>
            <p>No backups yet — they’re created automatically before any mod install.</p>
          </div>
        ) : (
          <div className={styles.installedList}>
            {backups.map((b) => (
              <div key={b.id} className={styles.installedRow}>
                <div className={styles.installedName}>
                  <strong>{new Date(b.created_at).toLocaleString()}</strong>
                  <span className={styles.muted}>
                    {(b.size_bytes / (1024 * 1024)).toFixed(1)} MB · {b.note || b.id}
                  </span>
                </div>
                <button className="btn btn-primary" onClick={() => onRestore(b)}>
                  Roll back to here
                </button>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
