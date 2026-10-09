import { useEffect, useMemo, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import GameCard from "../components/GameCard";
import Icon from "../components/Icon";
import LauncherBadge, { launcherLabel } from "../components/LauncherBadge";
import LightPlane from "../components/LightPlane";
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
    view,
    setView,
    rescan,
    scanning,
    fetchingCovers,
    refreshCovers,
    load,
  } = useLibrary();
  const push = useToast((s) => s.push);
  const [withTrainer, setWithTrainer] = useState<Set<string>>(new Set());
  const [onlyTrainers, setOnlyTrainers] = useState(false);

  useEffect(() => {
    if (!games.length) return;
    api
      .gamesWithTrainers(games.map((g) => ({ id: g.id, name: g.name, exe_path: g.exe_path })))
      .then((ids) => setWithTrainer(new Set(ids)))
      .catch(() => {});
  }, [games]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return games.filter((g) => {
      if (filterLauncher && g.launcher !== filterLauncher) return false;
      if (onlyTrainers && !withTrainer.has(g.id)) return false;
      if (q && !fuzzy(g.name.toLowerCase(), q)) return false;
      return true;
    });
  }, [games, query, filterLauncher, onlyTrainers, withTrainer]);

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
    push(`Scan finished: ${added} new ${added === 1 ? "game" : "games"}`, "success");
  };

  const launchers = LAUNCHERS.filter((l) => games.some((g) => g.launcher === l));

  return (
    <div className={styles.page}>
      <section className={styles.hero}>
        <LightPlane className={styles.heroLight} />
        <div className={styles.heroText}>
          <span className="label-caps">My games</span>
          <h1 className="display-lg">Your trainers. No time limit.</h1>
          <p className={styles.heroSub}>
            {games.length} {games.length === 1 ? "game" : "games"} found
            {withTrainer.size > 0 && ` · ${withTrainer.size} with a trainer`}
          </p>
        </div>
      </section>

      <div className={styles.toolbar}>
        <div className={styles.chips} role="group" aria-label="Filter by launcher">
          <button
            className={`${styles.chip} ${!filterLauncher ? styles.chipOn : ""}`}
            aria-pressed={!filterLauncher}
            onClick={() => setFilterLauncher(null)}
          >
            All
          </button>
          {launchers.map((l) => (
            <button
              key={l}
              className={`${styles.chip} ${filterLauncher === l ? styles.chipOn : ""}`}
              aria-pressed={filterLauncher === l}
              onClick={() => setFilterLauncher(filterLauncher === l ? null : l)}
            >
              {launcherLabel(l)}
            </button>
          ))}
        </div>

        <div className={styles.toolbarRight}>
          <div className="ag-seg" role="group" aria-label="Which games">
            <button className={!onlyTrainers ? "is-on" : ""} aria-pressed={!onlyTrainers} onClick={() => setOnlyTrainers(false)}>
              All
            </button>
            <button className={onlyTrainers ? "is-on" : ""} aria-pressed={onlyTrainers} onClick={() => setOnlyTrainers(true)}>
              With trainer
            </button>
          </div>
          <div className="ag-seg" role="group" aria-label="Layout">
            <button className={view === "grid" ? "is-on" : ""} aria-pressed={view === "grid"} onClick={() => setView("grid")}>
              Grid
            </button>
            <button className={view === "list" ? "is-on" : ""} aria-pressed={view === "list"} onClick={() => setView("list")}>
              List
            </button>
          </div>
          <button className="ag-btn" onClick={onAddManual}>
            <Icon name="plus" />
            Add game
          </button>
          <button
            className="ag-icon-btn"
            onClick={() => refreshCovers(true)}
            disabled={fetchingCovers || games.length === 0}
            aria-label="Refresh cover art"
            title={fetchingCovers ? "Fetching cover art" : "Refresh cover art"}
          >
            <Icon name="image" />
          </button>
        </div>
      </div>

      {filtered.length === 0 ? (
        <EmptyState games={games} onRescan={onRescan} scanning={scanning} />
      ) : view === "grid" ? (
        <div className={styles.grid}>
          {filtered.map((g) => (
            <GameCard key={g.id} game={g} hasTrainer={withTrainer.has(g.id)} />
          ))}
        </div>
      ) : (
        <div className={styles.list}>
          {filtered.map((g) => (
            <ListRow key={g.id} game={g} hasTrainer={withTrainer.has(g.id)} />
          ))}
        </div>
      )}
    </div>
  );
}

function ListRow({ game, hasTrainer }: { game: GameRow; hasTrainer: boolean }) {
  return (
    <a className={styles.row} href={`#/game/${encodeURIComponent(game.id)}`}>
      <div className={styles.rowName}>{game.name}</div>
      <div className={styles.rowTags}>
        {hasTrainer && <span className="ag-chip ag-chip--ink">Trainer</span>}
        <LauncherBadge launcher={game.launcher as Launcher} />
      </div>
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
        <h2 className="title">No games yet</h2>
        <p>Scan to find games from Steam, Epic, GOG, EA, Ubisoft, Xbox and Rockstar, or add a folder yourself.</p>
        <button className="ag-btn ag-btn--primary" onClick={onRescan} disabled={scanning}>
          <Icon name="refresh" />
          {scanning ? "Scanning" : "Scan for games"}
        </button>
      </div>
    );
  }
  return (
    <div className={styles.empty}>
      <h2 className="title">No matches</h2>
      <p>Change the filters or search to see more games.</p>
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
