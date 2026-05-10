import { create } from "zustand";
import { dbq, GameRow } from "../api/db";
import { api, DetectedGame, Launcher } from "../api/tauri";

const COVER_CONCURRENCY = 4;
const COVER_NEGATIVE_TTL_MS = 24 * 60 * 60 * 1000;
const NEGATIVE_KEY = "__none__";

function gameKey(g: DetectedGame): string {
  return `${g.launcher}:${g.app_id ?? g.install_path}`;
}

function detectedToRow(g: DetectedGame): GameRow {
  return {
    id: gameKey(g),
    name: g.name,
    launcher: g.launcher,
    install_path: g.install_path,
    exe_path: g.exe_path,
    app_id: g.app_id,
    cover_url: null,
    last_played: null,
    playtime_secs: 0,
    created_at: new Date().toISOString(),
  };
}

interface LibraryState {
  games: GameRow[];
  loading: boolean;
  scanning: boolean;
  fetchingCovers: boolean;
  query: string;
  filterLauncher: Launcher | null;
  filterModded: boolean;
  view: "grid" | "list";

  load: () => Promise<void>;
  rescan: () => Promise<{ added: number }>;
  refreshCovers: (force?: boolean) => Promise<void>;
  setQuery: (q: string) => void;
  setFilterLauncher: (l: Launcher | null) => void;
  setFilterModded: (m: boolean) => void;
  setView: (v: "grid" | "list") => void;
  removeGame: (id: string) => Promise<void>;
}

export const useLibrary = create<LibraryState>((set, get) => ({
  games: [],
  loading: false,
  scanning: false,
  fetchingCovers: false,
  query: "",
  filterLauncher: null,
  filterModded: false,
  view: "grid",

  async load() {
    set({ loading: true });
    try {
      const games = await dbq.listGames();
      set({ games, loading: false });
      get().refreshCovers().catch((e) => console.warn("cover backfill failed", e));
    } catch (e) {
      console.error("library load failed", e);
      set({ loading: false });
    }
  },

  async rescan() {
    set({ scanning: true });
    let added = 0;
    try {
      const detected = await api.scanAllLibraries();
      const existing = new Set(get().games.map((g) => g.id));
      for (const g of detected) {
        const row = detectedToRow(g);
        if (!existing.has(row.id)) added++;
        await dbq.upsertGame(row);
      }
      const games = await dbq.listGames();
      set({ games, scanning: false });
      get().refreshCovers().catch((e) => console.warn("cover backfill failed", e));
    } catch (e) {
      console.error("rescan failed", e);
      set({ scanning: false });
    }
    return { added };
  },

  async refreshCovers(force = false) {
    if (get().fetchingCovers) return;
    set({ fetchingCovers: true });

    try {
      const games = get().games;
      const todo = games.filter((g) => {
        if (force) return true;
        if (!g.cover_url) return true;
        if (g.cover_url === NEGATIVE_KEY) {
          // Re-try after the negative-cache window expires.
          const ts = Number(g.last_played);
          if (!Number.isFinite(ts)) return true;
          return Date.now() - ts > COVER_NEGATIVE_TTL_MS;
        }
        return false;
      });
      if (todo.length === 0) return;

      let cursor = 0;
      const workers: Promise<void>[] = [];
      const next = async () => {
        while (cursor < todo.length) {
          const g = todo[cursor++];
          try {
            const url = await api.fetchCoverArt(
              g.name,
              g.launcher as Launcher,
              g.app_id
            );
            const finalUrl = url ?? NEGATIVE_KEY;
            await dbq.setCoverUrl(g.id, finalUrl);
            set({
              games: get().games.map((x) =>
                x.id === g.id
                  ? {
                      ...x,
                      cover_url: finalUrl === NEGATIVE_KEY ? null : finalUrl,
                    }
                  : x
              ),
            });
          } catch (e) {
            console.warn(`cover lookup failed for ${g.name}`, e);
          }
        }
      };
      for (let i = 0; i < COVER_CONCURRENCY; i++) workers.push(next());
      await Promise.all(workers);
    } finally {
      set({ fetchingCovers: false });
    }
  },

  setQuery: (query) => set({ query }),
  setFilterLauncher: (l) => set({ filterLauncher: l }),
  setFilterModded: (m) => set({ filterModded: m }),
  setView: (v) => set({ view: v }),

  async removeGame(id: string) {
    await dbq.deleteGame(id);
    set({ games: get().games.filter((g) => g.id !== id) });
  },
}));
