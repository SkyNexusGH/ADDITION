import { useNavigate } from "react-router-dom";
import { GameRow } from "../api/db";
import LauncherBadge from "./LauncherBadge";
import { Launcher } from "../api/tauri";
import styles from "./GameCard.module.css";

interface Props {
  game: GameRow;
  modCount?: number;
  hasTrainer?: boolean;
}

export default function GameCard({ game, modCount = 0, hasTrainer = false }: Props) {
  const navigate = useNavigate();
  const initial = game.name.trim().charAt(0).toUpperCase() || "?";

  return (
    <button
      className={styles.card}
      onClick={() => navigate(`/game/${encodeURIComponent(game.id)}`)}
      title={game.name}
    >
      <div className={styles.cover}>
        {game.cover_url ? (
          <img src={game.cover_url} alt={game.name} loading="lazy" />
        ) : (
          <div className={styles.placeholder}>
            <span>{initial}</span>
          </div>
        )}
        <div className={styles.gradient} />
        <div className={styles.topRow}>
          <LauncherBadge launcher={game.launcher as Launcher} />
        </div>
        <div className={styles.pills}>
          {modCount > 0 && (
            <span className={styles.pill}>
              {modCount} mod{modCount === 1 ? "" : "s"}
            </span>
          )}
          {hasTrainer && <span className={`${styles.pill} ${styles.trainer}`}>Trainer</span>}
        </div>
      </div>
      <div className={styles.title}>{game.name}</div>
    </button>
  );
}
