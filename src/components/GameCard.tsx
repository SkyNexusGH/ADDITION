import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { GameRow } from "../api/db";
import { Launcher } from "../api/tauri";
import LauncherBadge from "./LauncherBadge";
import styles from "./GameCard.module.css";

interface Props {
  game: GameRow;
  hasTrainer?: boolean;
}

export default function GameCard({ game, hasTrainer = false }: Props) {
  const navigate = useNavigate();
  const initial = game.name.trim().charAt(0).toUpperCase() || "?";
  const [coverFailed, setCoverFailed] = useState(false);

  return (
    <button
      className={styles.card}
      onClick={() => navigate(`/game/${encodeURIComponent(game.id)}`)}
      title={game.name}
    >
      <div className={styles.cover}>
        {game.cover_url && !coverFailed ? (
          <img src={game.cover_url} alt="" loading="lazy" onError={() => setCoverFailed(true)} />
        ) : (
          <div className={`${styles.placeholder} ag-grain`} aria-hidden="true">
            <span>{initial}</span>
          </div>
        )}
        {hasTrainer && <span className={`ag-chip ag-chip--ink ${styles.trainer}`}>Trainer</span>}
      </div>
      <div className={styles.meta}>
        <span className={styles.title}>{game.name}</span>
        <LauncherBadge launcher={game.launcher as Launcher} />
      </div>
    </button>
  );
}
