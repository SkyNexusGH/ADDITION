import { useEffect, useState } from "react";
import { GameRow } from "../../api/db";
import { api, TrainerEntry } from "../../api/tauri";
import styles from "./Tabs.module.css";

const ANTICHEAT_KEYWORDS = [
  "valorant", "fortnite", "apex", "rainbow six", "battlefield", "destiny",
  "easy anti", "battleye", "vanguard", "ricochet", "warzone", "rust ",
  "pubg", "tarkov", "the finals",
];

function isAnticheatGame(name: string): boolean {
  const lower = name.toLowerCase();
  return ANTICHEAT_KEYWORDS.some((kw) => lower.includes(kw));
}

export default function TrainersTab({ game }: { game: GameRow }) {
  const [trainers, setTrainers] = useState<TrainerEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const anticheat = isAnticheatGame(game.name);

  useEffect(() => {
    setTrainers(null);
    setError(null);
    api
      .listTrainers(game.name)
      .then(setTrainers)
      .catch((e) => setError(String(e)));
  }, [game.name]);

  if (error) {
    return (
      <div className={styles.empty}>
        <h3>Trainer index unreachable</h3>
        <code className={styles.error}>{error}</code>
      </div>
    );
  }

  if (trainers === null) {
    return <div className={styles.muted}>Loading trainers…</div>;
  }

  return (
    <div className={styles.tab}>
      <div className={styles.disclaimer}>
        <strong>Single-player only.</strong> Trainers modify game memory and will trigger
        anti-cheat in multiplayer modes — using one online can result in a permanent ban.
        ADDITION never recommends modifying online games.
      </div>

      {anticheat && (
        <div className={`${styles.disclaimer} ${styles.danger}`}>
          <strong>This game runs anti-cheat (EAC / BattlEye / Vanguard / similar).</strong>{" "}
          Trainer use will likely result in a ban. Trainer launches are disabled.
        </div>
      )}

      {trainers.length === 0 ? (
        <div className={styles.empty}>
          <h3>No trainers indexed for "{game.name}"</h3>
          <p>
            ADDITION's trainer index is community-maintained at{" "}
            <a
              href="#"
              onClick={(e) => {
                e.preventDefault();
                api.openPath("https://github.com/addition-app/trainers");
              }}
            >
              github.com/addition-app/trainers
            </a>
            . If a free trainer exists for this game, open a PR to add it — or send a link to
            the maintainers and we'll add it for you. We don't ship made-up data: when nothing
            credible is indexed, the page stays empty.
          </p>
        </div>
      ) : (
        <div className={styles.trainerList}>
          {trainers.map((t) => (
            <article key={t.id} className={styles.trainerCard}>
              <div className={styles.trainerHeader}>
                <div>
                  <h4>{t.trainer}</h4>
                  <div className={styles.trainerMeta}>
                    <span>via {t.source}</span>
                    {t.last_updated && (
                      <>
                        <span>·</span>
                        <span>updated {t.last_updated}</span>
                      </>
                    )}
                  </div>
                </div>
                {t.anticheat_risk && (
                  <span className={styles.warnBadge}>Anti-cheat risk</span>
                )}
              </div>
              {t.features.length > 0 && (
                <ul className={styles.featureList}>
                  {t.features.map((f) => (
                    <li key={f}>{f}</li>
                  ))}
                </ul>
              )}
              <div className={styles.trainerActions}>
                <button
                  className="btn btn-primary"
                  disabled={anticheat}
                  onClick={() => api.openPath(t.url)}
                >
                  Open trainer page
                </button>
              </div>
            </article>
          ))}
        </div>
      )}
    </div>
  );
}
