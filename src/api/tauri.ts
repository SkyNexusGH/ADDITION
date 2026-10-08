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
};
