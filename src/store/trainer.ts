import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api, HostStatus, TrainerEntry } from "../api/tauri";

/**
 * The trainer the user has open. It keeps running (and keeps waiting for the
 * game to start) while they browse the rest of the app, like WeMod does.
 */
interface TrainerState {
  trainer: TrainerEntry | null;
  status: HostStatus | null;
  open: (t: TrainerEntry) => Promise<void>;
  close: () => Promise<void>;
  apply: (p: Promise<HostStatus>) => Promise<void>;
  /** Reload the trainer definition after it was edited on disk. */
  refresh: (t: TrainerEntry) => void;
}

let timer: number | null = null;

export const useTrainer = create<TrainerState>((set, get) => {
  const poll = async () => {
    if (!get().trainer) return;
    try {
      set({ status: await api.trainerPoll() });
    } catch {
      /* the next tick retries */
    }
  };

  // Hotkeys are handled in Rust; refresh right away when one fires.
  listen("trainer-updated", poll).catch(() => {});

  return {
    trainer: null,
    status: null,

    async open(t) {
      set({ trainer: t, status: await api.trainerOpen(t.id) });
      if (timer === null) timer = window.setInterval(poll, 1000);
    },

    async close() {
      if (timer !== null) window.clearInterval(timer);
      timer = null;
      await api.trainerClose();
      set({ trainer: null, status: null });
    },

    async apply(p) {
      set({ status: await p });
    },

    refresh(t) {
      if (get().trainer?.id === t.id) set({ trainer: t });
    },
  };
});
