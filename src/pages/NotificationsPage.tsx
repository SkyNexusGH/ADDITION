import { useToast } from "../store/toast";
import styles from "./NotificationsPage.module.css";

export default function NotificationsPage() {
  const history = useToast((s) => s.history);
  const clear = useToast((s) => s.clearHistory);

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h1>Notifications</h1>
        {history.length > 0 && (
          <button className="btn btn-ghost" onClick={clear}>Clear all</button>
        )}
      </div>

      {history.length === 0 ? (
        <div className={styles.empty}>
          <p>No notifications yet.</p>
        </div>
      ) : (
        <ul className={styles.list}>
          {history.map((t) => (
            <li key={t.id} className={`${styles.item} ${styles[t.variant]}`}>
              <span className={styles.time}>
                {new Date(t.ts).toLocaleString()}
              </span>
              <span>{t.message}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
