import { useMemo } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import GameCard from "../components/GameCard";
import LauncherBadge from "../components/LauncherBadge";
import { useLibrary } from "../store/library";
import { useToast } from "../store/toast";
import { dbq, GameRow } from "../api/db";
import { api, Launcher } from "../api/tauri";
import styles from "./LibraryPage.module.css";

const LAUNCHERS: Launcher[] = ["steam", "epic", "gog", "ea", "ubisoft", "xbox", "rockstar", "manual"];

export default function LibraryPage() {
  const {
    games,
    query,
    filterLauncher,
    setFilterLauncher,
    filterModded,
    setFilterModded,
    view,
    setView,
    rescan,
    scanning,
    fetchingCovers,
    refreshCovers,
    load,
  } = useLibrary();
  const push = useToast((s) => s.push);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return games.filter((g) => {
      if (filterLauncher && g.launcher !== filterLauncher) return false;
      if (q && !fuzzy(g.name.toLowerCase(), q)) return false;
      return true;
    });
  }, [games, query, filterLauncher, filterModded]);

  const onAddManual = async () => {
    const dir = await openDialog({ directory: true, multiple: false });
    if (!dir || Array.isArray(dir)) return;
    const name = window.prompt("Game name?", basename(dir as string)) ?? "";
    if (!name.trim()) return;
    const detected = await api.addManualGame(name.trim(), dir as string);
    const id = `${detected.launcher}:${detected.app_id ?? detected.install_path}`;
    await dbq.upsertGame({
      id,
      name: detected.name,
      launcher: detected.launcher,
      install_path: detected.install_path,
      exe_path: detected.exe_path,
      app_id: detected.app_id,
      cover_url: null,
      last_played: null,
      playtime_secs: 0,
      created_at: new Date().toISOString(),
    });
    await load();
    push(`Added ${detected.name}`, "success");
  };

  const onRescan = async () => {
    const { added } = await rescan();
    push(`Scan complete — ${added} new ${added === 1 ? "game" : "games"} added`, "success");
  };

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <div>
          <h1 className={styles.title}>Library</h1>
          <p className={styles.subtitle}>
            {games.length} {games.length === 1 ? "game" : "games"} detected
          </p>
        </div>
        <div className={styles.headerActions}>
          <button className="btn" onClick={onAddManual}>+ Add Manually</button>
          <button
            className="btn"
            onClick={() => refreshCovers(true)}
            disabled={fetchingCovers || games.length === 0}
            title="Re-fetch cover art for every game"
          >
            {fetchingCovers ? "Fetching art…" : "Refresh Covers"}
          </button>
          <button className="btn btn-primary" onClick={onRescan} disabled={scanning}>
            {scanning ? "Scanning…" : "Rescan All"}
          </button>
        </div>
      </div>

      <div className={styles.filters}>
        <div className={styles.filterChips}>
          <button
            className={`${styles.chip} ${!filterLauncher ? styles.chipActive : ""}`}
            onClick={() => setFilterLauncher(null)}
          >
            All
          </button>
          {LAUNCHERS.map((l) => (
            <button
              key={l}
              className={`${styles.chip} ${filterLauncher === l ? styles.chipActive : ""}`}
              onClick={() => setFilterLauncher(filterLauncher === l ? null : l)}
            >
              <LauncherBadge launcher={l} />
            </button>
          ))}
        </div>
        <div className={styles.viewToggle}>
          <button
            className={`${styles.toggleBtn} ${view === "grid" ? styles.toggleActive : ""}`}
            onClick={() => setView("grid")}
            title="Grid"
          >
            ▦
          </button>
          <button
            className={`${styles.toggleBtn} ${view === "list" ? styles.toggleActive : ""}`}
            onClick={() => setView("list")}
            title="List"
          >
            ☰
          </button>
        </div>
      </div>

      {filtered.length === 0 ? (
        <EmptyState games={games} onRescan={onRescan} scanning={scanning} />
      ) : view === "grid" ? (
        <div className={styles.grid}>
          {filtered.map((g) => (
            <GameCard key={g.id} game={g} />
          ))}
        </div>
      ) : (
        <div className={styles.list}>
          {filtered.map((g) => (
            <ListRow key={g.id} game={g} />
          ))}
        </div>
      )}
    </div>
  );
}

function ListRow({ game }: { game: GameRow }) {
  return (
    <a className={styles.row} href={`#/game/${encodeURIComponent(game.id)}`}>
      <div className={styles.rowName}>{game.name}</div>
      <LauncherBadge launcher={game.launcher as Launcher} />
      <div className={styles.rowPath}>{game.install_path}</div>
    </a>
  );
}

function EmptyState({
  games,
  onRescan,
  scanning,
}: {
  games: GameRow[];
  onRescan: () => void;
  scanning: boolean;
}) {
  if (games.length === 0) {
    return (
      <div className={styles.empty}>
        <h2>No games yet</h2>
        <p>
          Click <strong>Rescan All</strong> to detect games installed via Steam, Epic, GOG, EA,
          Ubisoft, Xbox, and Rockstar — or add a folder manually.
        </p>
        <button className="btn btn-primary" onClick={onRescan} disabled={scanning}>
          {scanning ? "Scanning…" : "Run First Scan"}
        </button>
      </div>
    );
  }
  return (
    <div className={styles.empty}>
      <h2>No matches</h2>
      <p>Adjust your filters or search to see more games.</p>
    </div>
  );
}

function fuzzy(haystack: string, needle: string): boolean {
  let i = 0;
  for (const ch of haystack) {
    if (ch === needle[i]) i++;
    if (i === needle.length) return true;
  }
  return false;
}

function basename(p: string): string {
  const parts = p.split(/[\\/]/);
  return parts[parts.length - 1] || parts[parts.length - 2] || "";
}
