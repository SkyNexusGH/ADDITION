import { useNavigate } from "react-router-dom";
import { useLibrary } from "../store/library";
import { useToast } from "../store/toast";
import Icon from "./Icon";
import styles from "./Topbar.module.css";

export default function Topbar() {
  const navigate = useNavigate();
  const { rescan, scanning, query, setQuery } = useLibrary();
  const push = useToast((s) => s.push);
  const history = useToast((s) => s.history);

  const onRescan = async () => {
    const { added } = await rescan();
    push(`Scan finished: ${added} new ${added === 1 ? "game" : "games"}`, "success");
  };

  return (
    <header className={styles.topbar}>
      <label className={styles.search}>
        <Icon name="search" />
        <input
          type="search"
          placeholder="Search your games"
          aria-label="Search your games"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </label>

      <div className={styles.actions}>
        <button className="ag-btn" onClick={onRescan} disabled={scanning}>
          <Icon name="refresh" />
          {scanning ? "Scanning" : "Rescan"}
        </button>
        <button
          className={`ag-icon-btn ${styles.bell}`}
          onClick={() => navigate("/notifications")}
          aria-label={history.length ? `Notifications, ${history.length} new` : "Notifications"}
          title="Notifications"
        >
          <Icon name="bell" />
          {history.length > 0 && <span className={styles.dot} />}
        </button>
      </div>
    </header>
  );
}
