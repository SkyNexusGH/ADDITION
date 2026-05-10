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

export interface InstalledMod {
  id: string;
  game_id: string;
  name: string;
  version: string;
  source: string;
  size_bytes: number;
  enabled: boolean;
  installed_at: string;
  backup_id: string | null;
}

export interface BackupEntry {
  id: string;
  game_id: string;
  created_at: string;
  size_bytes: number;
  path: string;
  note: string;
}

export interface TrainerEntry {
  id: string;
  game: string;
  trainer: string;
  source: string;
  url: string;
  features: string[];
  anticheat_risk: boolean;
  single_player_only: boolean;
  last_updated: string;
}

export const api = {
  scanAllLibraries: () => invoke<DetectedGame[]>("scan_all_libraries"),
  addManualGame: (name: string, install_path: string, exe_path?: string) =>
    invoke<DetectedGame>("add_manual_game", { name, installPath: install_path, exePath: exe_path }),
  launchGame: (launcher: Launcher, app_id?: string | null, exe_path?: string | null) =>
    invoke<void>("launch_game", { launcher, appId: app_id ?? null, exePath: exe_path ?? null }),
  installMod: (args: {
    game_id: string;
    target_dir: string;
    mod_name: string;
    version: string;
    source: string;
    url: string;
  }) =>
    invoke<InstalledMod>("install_mod", {
      gameId: args.game_id,
      targetDir: args.target_dir,
      modName: args.mod_name,
      version: args.version,
      source: args.source,
      url: args.url,
    }),
  uninstallMod: (game_id: string, target_dir: string, backup_id: string) =>
    invoke<void>("uninstall_mod", { gameId: game_id, targetDir: target_dir, backupId: backup_id }),
  listBackups: (game_id: string) =>
    invoke<BackupEntry[]>("list_backups", { gameId: game_id }),
  restoreBackup: (game_id: string, target_dir: string, backup_id: string) =>
    invoke<void>("restore_backup", { gameId: game_id, targetDir: target_dir, backupId: backup_id }),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  appDataDir: () => invoke<string>("app_data_dir"),
  fetchCoverArt: (name: string, launcher: Launcher, app_id: string | null) =>
    invoke<string | null>("fetch_cover_art", {
      name,
      launcher,
      appId: app_id,
    }),
  listTrainers: (game_name: string) =>
    invoke<TrainerEntry[]>("list_trainers", { gameName: game_name }),
};
