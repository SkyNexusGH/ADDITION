import { invoke } from "@tauri-apps/api/core";
import { dbq } from "./db";

export interface ModListing {
  id: string;
  name: string;
  author: string;
  description: string;
  thumbnail: string | null;
  downloads: number;
  updated_at: string;
  category: string;
  source: "curseforge" | "nexus";
  download_url: string | null;
  page_url: string;
  version: string;
}

const CACHE_TTL_MS = 24 * 60 * 60 * 1000;

async function searchCurseForge(gameName: string, key: string): Promise<ModListing[]> {
  return invoke<ModListing[]>("search_curseforge_mods", {
    gameName,
    apiKey: key,
  });
}

async function searchNexus(gameName: string, key: string): Promise<ModListing[]> {
  return invoke<ModListing[]>("search_nexus_mods", {
    gameName,
    apiKey: key,
  });
}

/**
 * Fetch the available mods for a game from every connected source.
 *
 * Resolution of game names → provider-specific IDs (CurseForge gameId, Nexus
 * domain) happens in Rust via each provider's games-list endpoint, with an
 * in-process cache. We only pass the human-readable name from here.
 */
export async function fetchModsForGame(
  gameId: string,
  gameName: string,
  options: { force?: boolean } = {}
): Promise<{ mods: ModListing[]; errors: string[] }> {
  if (!options.force) {
    const cached = (await dbq.getCachedMods(gameId, "all", CACHE_TTL_MS)) as
      | { mods: ModListing[]; errors: string[] }
      | ModListing[]
      | null;
    // Only honour the cache if it actually contains mods. Empty arrays from
    // a failed previous fetch must not poison the next 24 hours.
    if (cached) {
      const arr = Array.isArray(cached) ? cached : cached.mods;
      if (arr.length > 0) {
        return Array.isArray(cached) ? { mods: cached, errors: [] } : cached;
      }
    }
  }

  const bundledCfKey = (import.meta.env.VITE_CURSEFORGE_API_KEY as string | undefined) ?? "";
  const overrideCfKey = (await dbq.getSetting("api_curseforge")) ?? "";
  const cfKey = overrideCfKey || bundledCfKey;
  const nxKey = (await dbq.getSetting("api_nexus")) ?? "";

  const errors: string[] = [];
  const results: ModListing[] = [];

  if (cfKey) {
    try {
      results.push(...(await searchCurseForge(gameName, cfKey)));
    } catch (e: any) {
      const msg = `CurseForge: ${e?.toString?.() ?? e}`;
      console.warn(msg);
      errors.push(msg);
    }
  } else {
    errors.push("CurseForge: no API key bundled or configured.");
  }

  if (nxKey) {
    try {
      results.push(...(await searchNexus(gameName, nxKey)));
    } catch (e: any) {
      const msg = `Nexus: ${e?.toString?.() ?? e}`;
      console.warn(msg);
      errors.push(msg);
    }
  } else {
    errors.push("Nexus: not signed in.");
  }

  results.sort((a, b) => b.downloads - a.downloads);
  const payload = { mods: results, errors };
  // Only cache if at least one provider returned something useful.
  if (results.length > 0) {
    await dbq.putCachedMods(gameId, "all", payload);
  }
  return payload;
}
