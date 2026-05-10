import { useNavigate } from "react-router-dom";
import { useLibrary } from "../store/library";
import { useToast } from "../store/toast";
import styles from "./Topbar.module.css";

export default function Topbar() {
  const navigate = useNavigate();
  const { rescan, scanning, query, setQuery } = useLibrary();
  const push = useToast((s) => s.push);
  const history = useToast((s) => s.history);

  const onRescan = async () => {
    const { added } = await rescan();
    push(`Scan complete — ${added} new ${added === 1 ? "game" : "games"} added`, "success");
  };

  return (
    <div className={styles.topbar}>
      <div className={styles.search}>
        <span className={styles.searchIcon}>⌕</span>
        <input
          type="text"
          placeholder="Search your library…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>

      <div className={styles.actions}>
        <button className="btn" onClick={onRescan} disabled={scanning}>
          {scanning ? "Scanning…" : "Rescan Library"}
        </button>
        <button
          className="btn btn-ghost"
          onClick={() => navigate("/notifications")}
          title="Notifications"
        >
          ◐ {history.length > 0 ? <span className={styles.dot} /> : null}
        </button>
      </div>
    </div>
  );
}
