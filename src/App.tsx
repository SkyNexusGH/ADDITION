import { useEffect } from "react";
import { Route, Routes, Navigate } from "react-router-dom";
import Sidebar from "./components/Sidebar";
import Topbar from "./components/Topbar";
import ToastContainer from "./components/ToastContainer";
import LibraryPage from "./pages/LibraryPage";
import GameDetailPage from "./pages/GameDetailPage";
import SettingsPage from "./pages/SettingsPage";
import NotificationsPage from "./pages/NotificationsPage";
import { useLibrary } from "./store/library";
import styles from "./App.module.css";

export default function App() {
  const load = useLibrary((s) => s.load);

  useEffect(() => {
    load();
  }, [load]);

  return (
    <div className={styles.shell}>
      <Sidebar />
      <div className={styles.main}>
        <Topbar />
        <div className={styles.content}>
          <Routes>
            <Route path="/" element={<Navigate to="/library" replace />} />
            <Route path="/library" element={<LibraryPage />} />
            <Route path="/game/:id/*" element={<GameDetailPage />} />
            <Route path="/notifications" element={<NotificationsPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </div>
      </div>
      <ToastContainer />
    </div>
  );
}
