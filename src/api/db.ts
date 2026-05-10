import Database from "@tauri-apps/plugin-sql";

let _db: Database | null = null;

export async function db(): Promise<Database> {
  if (_db) return _db;
  _db = await Database.load("sqlite:addition.db");
  return _db;
}

export interface GameRow {
  id: string;
  name: string;
  launcher: string;
  install_path: string;
  exe_path: string | null;
  app_id: string | null;
  cover_url: string | null;
  last_played: string | null;
  playtime_secs: number;
  created_at: string;
}

export interface InstalledModRow {
  id: string;
  game_id: string;
  name: string;
  version: string;
  source: string;
  size_bytes: number;
  enabled: number;
  backup_id: string | null;
  installed_at: string;
}

export const dbq = {
  async upsertGame(row: GameRow) {
    const conn = await db();
    await conn.execute(
      `INSERT INTO games (id, name, launcher, install_path, exe_path, app_id, cover_url, last_played, playtime_secs)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
       ON CONFLICT(id) DO UPDATE SET
         name=excluded.name,
         launcher=excluded.launcher,
         install_path=excluded.install_path,
         exe_path=excluded.exe_path,
         app_id=excluded.app_id`,
      [
        row.id,
        row.name,
        row.launcher,
        row.install_path,
        row.exe_path,
        row.app_id,
        row.cover_url,
        row.last_played,
        row.playtime_secs,
      ]
    );
  },

  async listGames(): Promise<GameRow[]> {
    const conn = await db();
    return conn.select<GameRow[]>(
      `SELECT * FROM games ORDER BY name COLLATE NOCASE ASC`
    );
  },

  async deleteGame(id: string) {
    const conn = await db();
    await conn.execute(`DELETE FROM games WHERE id = $1`, [id]);
  },

  async setCoverUrl(id: string, coverUrl: string | null) {
    const conn = await db();
    await conn.execute(
      `UPDATE games SET cover_url = $1 WHERE id = $2`,
      [coverUrl, id]
    );
  },

  async getGame(id: string): Promise<GameRow | null> {
    const conn = await db();
    const rows = await conn.select<GameRow[]>(
      `SELECT * FROM games WHERE id = $1`,
      [id]
    );
    return rows[0] ?? null;
  },

  async insertInstalledMod(row: Omit<InstalledModRow, "installed_at">) {
    const conn = await db();
    await conn.execute(
      `INSERT INTO installed_mods (id, game_id, name, version, source, size_bytes, enabled, backup_id)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
      [
        row.id,
        row.game_id,
        row.name,
        row.version,
        row.source,
        row.size_bytes,
        row.enabled,
        row.backup_id,
      ]
    );
  },

  async listInstalledMods(gameId: string): Promise<InstalledModRow[]> {
    const conn = await db();
    return conn.select<InstalledModRow[]>(
      `SELECT * FROM installed_mods WHERE game_id = $1 ORDER BY installed_at DESC`,
      [gameId]
    );
  },

  async setModEnabled(id: string, enabled: boolean) {
    const conn = await db();
    await conn.execute(
      `UPDATE installed_mods SET enabled = $1 WHERE id = $2`,
      [enabled ? 1 : 0, id]
    );
  },

  async deleteInstalledMod(id: string) {
    const conn = await db();
    await conn.execute(`DELETE FROM installed_mods WHERE id = $1`, [id]);
  },

  async getSetting(key: string): Promise<string | null> {
    const conn = await db();
    const rows = await conn.select<{ value: string }[]>(
      `SELECT value FROM settings WHERE key = $1`,
      [key]
    );
    return rows[0]?.value ?? null;
  },

  async setSetting(key: string, value: string) {
    const conn = await db();
    await conn.execute(
      `INSERT INTO settings (key, value) VALUES ($1, $2)
       ON CONFLICT(key) DO UPDATE SET value = excluded.value`,
      [key, value]
    );
  },

  async getCachedMods(gameId: string, source: string, ttlMs: number) {
    const conn = await db();
    const rows = await conn.select<{ payload: string; fetched_at: string }[]>(
      `SELECT payload, fetched_at FROM mod_cache WHERE game_id = $1 AND source = $2`,
      [gameId, source]
    );
    if (!rows[0]) return null;
    const fetched = new Date(rows[0].fetched_at).getTime();
    if (Date.now() - fetched > ttlMs) return null;
    try {
      return JSON.parse(rows[0].payload);
    } catch {
      return null;
    }
  },

  async putCachedMods(gameId: string, source: string, payload: unknown) {
    const conn = await db();
    const id = `${gameId}:${source}`;
    await conn.execute(
      `INSERT INTO mod_cache (id, game_id, source, payload)
       VALUES ($1, $2, $3, $4)
       ON CONFLICT(id) DO UPDATE SET payload=excluded.payload, fetched_at=CURRENT_TIMESTAMP`,
      [id, gameId, source, JSON.stringify(payload)]
    );
  },
};
