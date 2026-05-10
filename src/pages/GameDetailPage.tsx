import { useEffect, useState } from "react";
import { useParams, useNavigate, Routes, Route, NavLink, Navigate } from "react-router-dom";
import { dbq, GameRow } from "../api/db";
import { api, Launcher } from "../api/tauri";
import LauncherBadge from "../components/LauncherBadge";
import ModsTab from "./tabs/ModsTab";
import TrainersTab from "./tabs/TrainersTab";
import InstalledTab from "./tabs/InstalledTab";
import GameSettingsTab from "./tabs/GameSettingsTab";
import { useToast } from "../store/toast";
import styles from "./GameDetailPage.module.css";

export default function GameDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const push = useToast((s) => s.push);
  const [game, setGame] = useState<GameRow | null>(null);

  useEffect(() => {
    if (!id) return;
    dbq.getGame(decodeURIComponent(id)).then(setGame);
  }, [id]);

  const onLaunch = async () => {
    if (!game) return;
    try {
      await api.launchGame(game.launcher as Launcher, game.app_id, game.exe_path);
      push(`Launching ${game.name}…`, "info");
    } catch (e: any) {
      push(`Launch failed: ${e?.toString?.() ?? "unknown error"}`, "danger");
    }
  };

  if (!game) {
    return (
      <div className={styles.empty}>
        <p>Loading game…</p>
        <button className="btn" onClick={() => navigate("/library")}>← Back to Library</button>
      </div>
    );
  }

  const initial = game.name.charAt(0).toUpperCase();

  return (
    <div className={styles.page}>
      <div className={styles.hero}>
        {game.cover_url && (
          <div
            className={styles.heroBlur}
            style={{ backgroundImage: `url(${game.cover_url})` }}
          />
        )}
        <div className={styles.heroOverlay} />
        <button className={styles.backBtn} onClick={() => navigate("/library")}>
          ← Library
        </button>
        <div className={styles.heroBody}>
          <div className={styles.cover}>
            {game.cover_url ? (
              <img src={game.cover_url} alt={game.name} />
            ) : (
              <div className={styles.coverPlaceholder}>{initial}</div>
            )}
          </div>
          <div className={styles.titleBlock}>
            <LauncherBadge launcher={game.launcher as Launcher} />
            <h1 className={styles.title}>{game.name}</h1>
            <div className={styles.path} title={game.install_path}>
              {game.install_path}
            </div>
            <div className={styles.actions}>
              <button className={styles.bigPlay} onClick={onLaunch}>
                ▶ Play
              </button>
              <button
                className="btn"
                onClick={() => api.openPath(game.install_path)}
              >
                Open Folder
              </button>
            </div>
          </div>
        </div>
      </div>

      <div className={styles.tabs}>
        {[
          { to: "mods", label: "Mods" },
          { to: "trainers", label: "Trainers" },
          { to: "installed", label: "Installed" },
          { to: "settings", label: "Settings" },
        ].map((t) => (
          <NavLink
            key={t.to}
            to={t.to}
            className={({ isActive }) =>
              `${styles.tab} ${isActive ? styles.tabActive : ""}`
            }
          >
            {t.label}
          </NavLink>
        ))}
      </div>

      <div className={styles.tabContent}>
        <Routes>
          <Route path="/" element={<Navigate to="mods" replace />} />
          <Route path="mods" element={<ModsTab game={game} />} />
          <Route path="trainers" element={<TrainersTab game={game} />} />
          <Route path="installed" element={<InstalledTab game={game} />} />
          <Route path="settings" element={<GameSettingsTab game={game} />} />
        </Routes>
      </div>
    </div>
  );
}
