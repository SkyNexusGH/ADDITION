import { useToast } from "../store/toast";
import styles from "./NotificationsPage.module.css";

const WORD = { info: "Info", success: "Done", warning: "Heads up", danger: "Error" } as const;

export default function NotificationsPage() {
  const history = useToast((s) => s.history);
  const clear = useToast((s) => s.clearHistory);

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div>
          <span className="label-caps">Activity</span>
          <h1 className="display-md">Notifications</h1>
        </div>
        {history.length > 0 && (
          <button className="ag-btn ag-btn--ghost" onClick={clear}>
            Clear all
          </button>
        )}
      </header>

      {history.length === 0 ? (
        <div className={styles.empty}>Nothing here yet.</div>
      ) : (
        <ul className={styles.list}>
          {history.map((t) => (
            <li key={t.id} className={`${styles.item} ${styles[t.variant]}`}>
              <div className={styles.meta}>
                <span className={styles.word}>{WORD[t.variant]}</span>
                <time className={styles.time}>{new Date(t.ts).toLocaleString()}</time>
              </div>
              <span>{t.message}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
