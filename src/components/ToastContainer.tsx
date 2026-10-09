import { useToast } from "../store/toast";
import styles from "./ToastContainer.module.css";

const WORD = { info: "Info", success: "Done", warning: "Heads up", danger: "Error" } as const;

export default function ToastContainer() {
  const toasts = useToast((s) => s.toasts);
  const dismiss = useToast((s) => s.dismiss);

  return (
    <div className={styles.stack} role="status" aria-live="polite">
      {toasts.map((t) => (
        <button key={t.id} className={`${styles.toast} ${styles[t.variant]}`} onClick={() => dismiss(t.id)}>
          <span className={styles.word}>{WORD[t.variant]}</span>
          <span>{t.message}</span>
        </button>
      ))}
    </div>
  );
}
