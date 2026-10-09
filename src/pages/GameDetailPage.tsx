import { useEffect, useState } from "react";
import { useParams, useNavigate, Routes, Route, NavLink, Navigate } from "react-router-dom";
import { dbq, GameRow } from "../api/db";
import { api, Launcher } from "../api/tauri";
import Icon from "../components/Icon";
import LauncherBadge from "../components/LauncherBadge";
import LightPlane from "../components/LightPlane";
import GameSettingsTab from "./tabs/GameSettingsTab";
import TrainerTab from "./tabs/TrainerTab";
import ScannerTab from "./tabs/ScannerTab";
import { useToast } from "../store/toast";
import styles from "./GameDetailPage.module.css";

export default function GameDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const push = useToast((s) => s.push);
  const [game, setGame] = useState<GameRow | null>(null);
  const [coverFailed, setCoverFailed] = useState(false);

  useEffect(() => {
    if (!id) return;
    dbq.getGame(decodeURIComponent(id)).then(setGame);
  }, [id]);

  const onLaunch = async () => {
    if (!game) return;
    try {
      await api.launchGame(game.launcher as Launcher, game.app_id, game.exe_path);
      push(`Starting ${game.name}`, "info");
    } catch (e: any) {
      push(`Couldn't start the game: ${e?.toString?.() ?? "unknown error"}`, "danger");
    }
  };

  if (!game) {
    return (
      <div className={styles.empty}>
        <p>Loading game</p>
        <button className="ag-btn" onClick={() => navigate("/library")}>
          <Icon name="back" />
          Library
        </button>
      </div>
    );
  }

  const initial = game.name.charAt(0).toUpperCase();

  return (
    <div className={styles.page}>
      <section className={styles.hero}>
        <LightPlane className={styles.heroLight} start={64} />
        <button className={`ag-btn ag-btn--sm ${styles.back}`} onClick={() => navigate("/library")}>
          <Icon name="back" />
          Library
        </button>
        <div className={styles.heroBody}>
          <div className={styles.cover}>
            {game.cover_url && !coverFailed ? (
              <img src={game.cover_url} alt="" onError={() => setCoverFailed(true)} />
            ) : (
              <div className={`${styles.coverPlaceholder} ag-grain`} aria-hidden="true">
                {initial}
              </div>
            )}
          </div>
          <div className={styles.titleBlock}>
            <LauncherBadge launcher={game.launcher as Launcher} />
            <h1 className={`display-lg ${styles.title}`}>{game.name}</h1>
            <div className={styles.path} title={game.install_path}>
              {game.install_path}
            </div>
            <div className={styles.actions}>
              <button className="ag-btn ag-btn--ember ag-btn--lg" onClick={onLaunch}>
                <Icon name="play" />
                Play
              </button>
              <button className="ag-btn ag-btn--lg" onClick={() => api.openPath(game.install_path)}>
                <Icon name="folder" />
                Open folder
              </button>
            </div>
          </div>
        </div>
      </section>

      <nav className={`ag-seg ${styles.tabs}`} aria-label="Game sections">
        {[
          { to: "trainer", label: "Trainer" },
          { to: "scanner", label: "Scanner" },
          { to: "settings", label: "Settings" },
        ].map((t) => (
          <NavLink key={t.to} to={t.to} className={({ isActive }) => `${styles.tab} ${isActive ? "is-on" : ""}`}>
            {t.label}
          </NavLink>
        ))}
      </nav>

      <div className={styles.tabContent}>
        <Routes>
          <Route path="/" element={<Navigate to="trainer" replace />} />
          <Route path="trainer" element={<TrainerTab game={game} />} />
          <Route path="scanner" element={<ScannerTab game={game} />} />
          <Route path="settings" element={<GameSettingsTab game={game} />} />
        </Routes>
      </div>
    </div>
  );
}
