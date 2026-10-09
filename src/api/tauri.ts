import { invoke } from "@tauri-apps/api/core";

export type Launcher =
  | "steam"
  | "epic"
  | "gog"
  | "ea"
  | "ubisoft"
  | "xbox"
  | "rockstar"
  | "manual";

export interface DetectedGame {
  name: string;
  install_path: string;
  launcher: Launcher;
  exe_path: string | null;
  app_id: string | null;
}

// Trainer engine types (mirror engine/src/*.rs) ------------------------------

export type ValueType = "u8" | "i16" | "i32" | "i64" | "f32" | "f64";
export const VALUE_TYPES: ValueType[] = ["i32", "f32", "i64", "f64", "i16", "u8"];

/** Offsets are hex strings like "0x5578"; plain numbers are accepted too. */
export interface PointerPath {
  module?: string | null;
  base: string | number;
  offsets?: (string | number)[];
}

interface CheatBase {
  id: string;
  name: string;
  description?: string;
  hotkey?: string | null;
}

export type Cheat =
  | (CheatBase & { kind: "freeze"; target: PointerPath; type: ValueType; value: number })
  | (CheatBase & {
      kind: "set";
      target: PointerPath;
      type: ValueType;
      min?: number;
      max?: number;
      default?: number;
    })
  | (CheatBase & { kind: "patch"; module?: string; aob: string; offset?: number; bytes: string });

export interface Trainer {
  schema?: number;
  id: string;
  game: string;
  aliases?: string[];
  process: string[];
  game_version?: string;
  author?: string;
  notes?: string;
  verified?: boolean;
  cheats: Cheat[];
}

export interface TrainerEntry extends Trainer {
  origin: "bundled" | "user";
  path: string | null;
}

export interface CheatState {
  id: string;
  active: boolean;
  value: number | null;
  error: string | null;
}

export type Phase = "idle" | "waiting" | "attached" | "blocked" | "error";

export interface HostStatus {
  trainer_id: string | null;
  phase: Phase;
  message: string | null;
  pid: number | null;
  cheats: CheatState[];
  hotkey_errors: string[];
}

export interface ProcessInfo {
  pid: number;
  name: string;
  parent_pid: number;
}

export type ScanFilter =
  | { kind: "exact"; value: string }
  | { kind: "unknown" | "changed" | "unchanged" | "increased" | "decreased" };

export interface ScanHit {
  address: number;
  value: number;
  module_offset: [string, number] | null;
}

export interface ScanSummary {
  count: number;
  hits: ScanHit[];
}

export interface PathView {
  path: PointerPath;
  display: string;
}

export const api = {
  scanAllLibraries: () => invoke<DetectedGame[]>("scan_all_libraries"),
  addManualGame: (name: string, install_path: string, exe_path?: string) =>
    invoke<DetectedGame>("add_manual_game", { name, installPath: install_path, exePath: exe_path }),
  launchGame: (launcher: Launcher, app_id?: string | null, exe_path?: string | null) =>
    invoke<void>("launch_game", { launcher, appId: app_id ?? null, exePath: exe_path ?? null }),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  appDataDir: () => invoke<string>("app_data_dir"),
  fetchCoverArt: (name: string, launcher: Launcher, app_id: string | null) =>
    invoke<string | null>("fetch_cover_art", {
      name,
      launcher,
      appId: app_id,
    }),

  // Trainer files
  listTrainers: () => invoke<{ trainers: TrainerEntry[]; problems: string[] }>("list_trainers"),
  trainersForGame: (gameName: string, exePath: string | null) =>
    invoke<TrainerEntry[]>("trainers_for_game", { gameName, exePath }),
  gamesWithTrainers: (games: { id: string; name: string; exe_path: string | null }[]) =>
    invoke<string[]>("games_with_trainers", { games }),
  saveTrainer: (trainer: Trainer) => invoke<string>("save_trainer", { trainer }),
  deleteTrainer: (id: string) => invoke<void>("delete_trainer", { id }),

  // Running a trainer
  trainerOpen: (id: string) => invoke<HostStatus>("trainer_open", { id }),
  trainerPoll: () => invoke<HostStatus>("trainer_poll"),
  trainerClose: () => invoke<void>("trainer_close"),
  cheatEnable: (id: string, value?: number | null) =>
    invoke<HostStatus>("cheat_enable", { id, value: value ?? null }),
  cheatDisable: (id: string) => invoke<HostStatus>("cheat_disable", { id }),
  cheatSet: (id: string, value: number) => invoke<HostStatus>("cheat_set", { id, value }),

  // Memory scanner
  listProcesses: () => invoke<ProcessInfo[]>("list_processes"),
  scanOpen: (pid: number) => invoke<void>("scan_open", { pid }),
  scanClose: () => invoke<void>("scan_close"),
  scanRun: (first: boolean, valueType: ValueType, filter: ScanFilter) =>
    invoke<ScanSummary>("scan_run", { first, valueType, filter }),
  scanResults: () => invoke<ScanSummary>("scan_results"),
  scanReset: () => invoke<void>("scan_reset"),
  scanProgress: () => invoke<number>("scan_progress"),
  scanCancel: () => invoke<void>("scan_cancel"),
  scanRead: (targets: { path: PointerPath; value_type: ValueType }[]) =>
    invoke<(number | null)[]>("scan_read", { targets }),
  scanWrite: (path: PointerPath, valueType: ValueType, value: number) =>
    invoke<void>("scan_write", { path, valueType, value }),
  scanFreeze: (key: string, path: PointerPath, valueType: ValueType, value: number | null) =>
    invoke<void>("scan_freeze", { key, path, valueType, value }),
  scanFindPointers: (address: number, maxDepth?: number) =>
    invoke<PathView[]>("scan_find_pointers", { address, maxDepth: maxDepth ?? null }),
  scanFilterPointers: (paths: PointerPath[], address: number) =>
    invoke<PathView[]>("scan_filter_pointers", { paths, address }),
};

export const hex = (n: number) => "0x" + n.toString(16).toUpperCase();

/** "[["game.exe"+0x10]+0x8]+0x4", the same notation the engine prints. */
export function formatPath(p: PointerPath): string {
  const h = (v: string | number) => (typeof v === "number" ? (v < 0 ? "-" + hex(-v) : hex(v)) : v);
  let s = p.module ? `"${p.module}"+${h(p.base)}` : h(p.base);
  for (const o of p.offsets ?? []) s = `[${s}]+${h(o)}`;
  return s;
}

export function pathForHit(hit: ScanHit): PointerPath {
  return hit.module_offset
    ? { module: hit.module_offset[0], base: hex(hit.module_offset[1]), offsets: [] }
    : { base: hex(hit.address), offsets: [] };
}

/** A path that doesn't start in a module points at the heap and won't survive a restart. */
export const isStatic = (p: PointerPath) => !!p.module;
