import { NavLink } from "react-router-dom";
import Icon, { IconName } from "./Icon";
import styles from "./Sidebar.module.css";

const NAV: { to: string; label: string; icon: IconName }[] = [
  { to: "/library", label: "Library", icon: "library" },
  { to: "/notifications", label: "Notifications", icon: "bell" },
  { to: "/settings", label: "Settings", icon: "settings" },
];

export default function Sidebar() {
  return (
    <aside className={styles.sidebar}>
      <div className={styles.brand}>
        <img src="/assets/ADDITION_Logo.png" alt="ADDITION" className={styles.logo} />
      </div>

      <nav className={styles.nav}>
        {NAV.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            title={item.label}
            aria-label={item.label}
            className={({ isActive }) => `ag-icon-btn ${styles.navLink} ${isActive ? "is-active" : ""}`}
          >
            <Icon name={item.icon} />
          </NavLink>
        ))}
      </nav>

      <div className={`caption ${styles.version}`}>v0.1</div>
    </aside>
  );
}
