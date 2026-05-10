import { useEffect, useState } from "react";
import { GameRow } from "../../api/db";
import { fetchModsForGame, ModListing } from "../../api/mods";
import { dbq } from "../../api/db";
import { api } from "../../api/tauri";
import { useToast } from "../../store/toast";
import styles from "./Tabs.module.css";

interface Props {
  game: GameRow;
}

type SortKey = "downloads" | "updated" | "name";

export default function ModsTab({ game }: Props) {
  const [mods, setMods] = useState<ModListing[]>([]);
  const [installedIds, setInstalledIds] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [errors, setErrors] = useState<string[]>([]);
  const [sort, setSort] = useState<SortKey>("downloads");
  const [category, setCategory] = useState<string>("All");
  const [installing, setInstalling] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const push = useToast((s) => s.push);

  const load = (force = false) => {
    setLoading(true);
    setErrors([]);
    fetchModsForGame(game.id, game.name, { force })
      .then((res) => {
        setMods(res.mods);
        setErrors(res.errors);
      })
      .catch((e) => setErrors([String(e)]))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    load(false);
    dbq.listInstalledMods(game.id).then((rows) => {
      setInstalledIds(new Set(rows.map((r) => `${r.source}:${r.name}`)));
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [game.id, game.name]);

  const categories = ["All", ...Array.from(new Set(mods.map((m) => m.category))).sort()];
  const filtered = mods
    .filter((m) => category === "All" || m.category === category)
    .filter((m) =>
      search.trim() === "" ? true : m.name.toLowerCase().includes(search.toLowerCase())
    )
    .sort((a, b) => {
      switch (sort) {
        case "downloads":
          return b.downloads - a.downloads;
        case "updated":
          return b.updated_at.localeCompare(a.updated_at);
        case "name":
          return a.name.localeCompare(b.name);
      }
    });

  const isInstalled = (m: ModListing) => installedIds.has(`${m.source}:${m.name}`);

  const onInstall = async (mod: ModListing) => {
    if (!mod.download_url) {
      push("This mod requires a manual download from its page.", "warning");
      await api.openPath(mod.page_url);
      return;
    }
    setInstalling(mod.id);
    try {
      const installed = await api.installMod({
        game_id: game.id,
        target_dir: game.install_path,
        mod_name: mod.name,
        version: mod.version,
        source: mod.source,
        url: mod.download_url,
      });
      await dbq.insertInstalledMod({
        id: installed.id,
        game_id: installed.game_id,
        name: installed.name,
        version: installed.version,
        source: installed.source,
        size_bytes: installed.size_bytes,
        enabled: 1,
        backup_id: installed.backup_id,
      });
      setInstalledIds((s) => new Set([...s, `${mod.source}:${mod.name}`]));
      push(`Installed ${mod.name}`, "success");
    } catch (e: any) {
      push(`Install failed: ${e?.toString?.()}`, "danger");
    } finally {
      setInstalling(null);
    }
  };

  if (loading) return <div className={styles.muted}>Loading mods…</div>;

  if (mods.length === 0) {
    return (
      <div className={styles.empty}>
        <h3>No mods found for "{game.name}"</h3>
        <p>
          ADDITION queries CurseForge and Nexus by game name. The error block below shows what
          each provider said — usually a name-match miss (Nexus uses "Skyrim Special Edition",
          your library shows "The Elder Scrolls V: Skyrim", etc.) or that the game simply isn't
          in either catalogue.
        </p>
        {errors.length > 0 && (
          <code className={styles.error}>
            {errors.map((e, i) => (
              <div key={i}>{e}</div>
            ))}
          </code>
        )}
        <button className="btn btn-primary" onClick={() => load(true)}>
          Try again (force refresh)
        </button>
      </div>
    );
  }

  return (
    <div className={styles.tab}>
      <div className={styles.modToolbar}>
        <input
          type="text"
          placeholder={`Search ${mods.length} mods…`}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className={styles.searchInput}
        />
        <select value={category} onChange={(e) => setCategory(e.target.value)}>
          {categories.map((c) => (
            <option key={c} value={c}>{c}</option>
          ))}
        </select>
        <select value={sort} onChange={(e) => setSort(e.target.value as SortKey)}>
          <option value="downloads">Most Downloaded</option>
          <option value="updated">Recently Updated</option>
          <option value="name">A → Z</option>
        </select>
        <button className="btn btn-ghost" onClick={() => load(true)} title="Refresh">↻</button>
        <span className={styles.muted}>{filtered.length} of {mods.length}</span>
      </div>

      {errors.length > 0 && (
        <code className={styles.error}>
          {errors.map((e, i) => (
            <div key={i}>{e}</div>
          ))}
        </code>
      )}

      <div className={styles.modRows}>
        {filtered.map((m) => {
          const installed = isInstalled(m);
          return (
            <article key={m.id} className={`${styles.modRow} ${installed ? styles.modRowInstalled : ""}`}>
              <div className={styles.modThumbSm}>
                {m.thumbnail ? (
                  <img src={m.thumbnail} alt="" />
                ) : (
                  <div className={styles.thumbFallback}>{m.source[0].toUpperCase()}</div>
                )}
              </div>
              <div className={styles.modInfo}>
                <div className={styles.modTitleLine}>
                  <h4>{m.name}</h4>
                  <span className={`${styles.sourceBadge} ${styles[`src_${m.source}`]}`}>
                    {m.source}
                  </span>
                </div>
                <p className={styles.modDescLine}>{m.description}</p>
                <div className={styles.modMetaLine}>
                  <span>by {m.author}</span>
                  <span className={styles.dot}>·</span>
                  <span>{m.downloads.toLocaleString()} downloads</span>
                  {m.category && (
                    <>
                      <span className={styles.dot}>·</span>
                      <span>{m.category}</span>
                    </>
                  )}
                </div>
              </div>
              <div className={styles.modAction}>
                {installed ? (
                  <span className={styles.installedTag}>✓ Installed</span>
                ) : (
                  <button
                    className={styles.installBtn}
                    onClick={() => onInstall(m)}
                    disabled={installing === m.id}
                  >
                    {installing === m.id ? "…" : "+ Install"}
                  </button>
                )}
                <button
                  className={styles.linkBtn}
                  onClick={() => api.openPath(m.page_url)}
                  title="Open mod page"
                >
                  ↗
                </button>
              </div>
            </article>
          );
        })}
      </div>
    </div>
  );
}
