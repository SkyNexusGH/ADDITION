import { create } from "zustand";
import { PathView, PointerPath, ScanSummary, ValueType } from "../api/tauri";

/** A value the user is keeping an eye on (Cheat Engine's "address list"). */
export interface WatchEntry {
  key: string;
  label: string;
  /** Where it was found in this run of the game. */
  address: number;
  path: PointerPath;
  type: ValueType;
  value: number | null;
  frozen: boolean;
}

interface ScannerState {
  pid: number | null;
  processName: string | null;
  valueType: ValueType;
  summary: ScanSummary | null;
  watch: WatchEntry[];
  pointers: { key: string; paths: PathView[] } | null;
  set: (p: Partial<ScannerState>) => void;
  updateEntry: (key: string, p: Partial<WatchEntry>) => void;
  removeEntry: (key: string) => void;
}

/** Survives tab switches so a half-finished search isn't lost. */
export const useScanner = create<ScannerState>((set) => ({
  pid: null,
  processName: null,
  valueType: "i32",
  summary: null,
  watch: [],
  pointers: null,
  set: (p) => set(p),
  updateEntry: (key, p) =>
    set((s) => ({ watch: s.watch.map((w) => (w.key === key ? { ...w, ...p } : w)) })),
  removeEntry: (key) =>
    set((s) => ({
      watch: s.watch.filter((w) => w.key !== key),
      pointers: s.pointers?.key === key ? null : s.pointers,
    })),
}));
