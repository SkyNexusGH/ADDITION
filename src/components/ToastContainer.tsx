import { useToast } from "../store/toast";
import styles from "./ToastContainer.module.css";

export default function ToastContainer() {
  const toasts = useToast((s) => s.toasts);
  const dismiss = useToast((s) => s.dismiss);

  return (
    <div className={styles.stack}>
      {toasts.map((t) => (
        <div
          key={t.id}
          className={`${styles.toast} ${styles[t.variant]}`}
          onClick={() => dismiss(t.id)}
        >
          <span>{t.message}</span>
        </div>
      ))}
    </div>
  );
}
