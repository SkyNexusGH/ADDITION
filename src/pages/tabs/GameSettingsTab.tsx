import { useEffect, useState } from "react";
import { GameRow, dbq } from "../../api/db";
import { useToast } from "../../store/toast";
import styles from "./Tabs.module.css";

export default function GameSettingsTab({ game }: { game: GameRow }) {
  const [launchArgs, setLaunchArgs] = useState("");
  const push = useToast((s) => s.push);

  useEffect(() => {
    dbq.getSetting(`launch_args:${game.id}`).then((v) => setLaunchArgs(v ?? ""));
  }, [game.id]);

  const save = async () => {
    await dbq.setSetting(`launch_args:${game.id}`, launchArgs);
    push("Saved", "success");
  };

  return (
    <div className={styles.tab}>
      <section className={styles.formSection}>
        <h3 className={styles.sectionTitle}>Per-game settings</h3>
        <label className={styles.field}>
          <span>Launch arguments</span>
          <input
            type="text"
            value={launchArgs}
            onChange={(e) => setLaunchArgs(e.target.value)}
            placeholder="-windowed -dx12"
          />
        </label>
        <label className={styles.field}>
          <span>Install path</span>
          <input type="text" value={game.install_path} readOnly />
        </label>
        <label className={styles.field}>
          <span>Launcher</span>
          <input type="text" value={game.launcher} readOnly />
        </label>
        <button className="btn btn-primary" onClick={save}>Save</button>
      </section>
    </div>
  );
}
