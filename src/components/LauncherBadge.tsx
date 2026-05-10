import { Launcher } from "../api/tauri";
import styles from "./LauncherBadge.module.css";

const META: Record<Launcher, { label: string; color: string; bg: string }> = {
  steam:    { label: "Steam",        color: "#9bc7ff", bg: "rgba(59, 130, 246, 0.16)" },
  xbox:     { label: "Xbox",         color: "#86efac", bg: "rgba(34, 197, 94, 0.14)"  },
  epic:     { label: "Epic",         color: "#cbd5e1", bg: "rgba(148, 163, 184, 0.16)" },
  gog:      { label: "GOG",          color: "#d8b4fe", bg: "rgba(168, 85, 247, 0.14)" },
  ea:       { label: "EA",           color: "#fdba74", bg: "rgba(245, 158, 11, 0.16)" },
  ubisoft:  { label: "Ubisoft",      color: "#7dd3fc", bg: "rgba(2, 132, 199, 0.18)"  },
  rockstar: { label: "Rockstar",     color: "#fde047", bg: "rgba(234, 179, 8, 0.18)"  },
  manual:   { label: "Manual",       color: "#94a3b8", bg: "rgba(100, 116, 139, 0.12)" },
};

export default function LauncherBadge({ launcher }: { launcher: Launcher }) {
  const m = META[launcher];
  return (
    <span
      className={styles.badge}
      style={{ color: m.color, backgroundColor: m.bg, borderColor: m.color }}
    >
      {m.label}
    </span>
  );
}
