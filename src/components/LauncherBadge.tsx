import { Launcher } from "../api/tauri";

const LABELS: Record<Launcher, string> = {
  steam: "Steam",
  xbox: "Xbox",
  epic: "Epic",
  gog: "GOG",
  ea: "EA",
  ubisoft: "Ubisoft",
  rockstar: "Rockstar",
  manual: "Manual",
};

export const launcherLabel = (l: Launcher) => LABELS[l] ?? l;

/** Neutral tag: the launcher's name carries the meaning, not a colour. */
export default function LauncherBadge({ launcher }: { launcher: Launcher }) {
  return <span className="ag-tag">{launcherLabel(launcher)}</span>;
}
