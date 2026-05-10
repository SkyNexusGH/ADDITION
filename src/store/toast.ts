import { create } from "zustand";

export type ToastVariant = "info" | "success" | "warning" | "danger";

export interface Toast {
  id: string;
  message: string;
  variant: ToastVariant;
  ts: number;
}

interface ToastState {
  toasts: Toast[];
  history: Toast[];
  push: (message: string, variant?: ToastVariant) => void;
  dismiss: (id: string) => void;
  clearHistory: () => void;
}

export const useToast = create<ToastState>((set, get) => ({
  toasts: [],
  history: [],
  push(message, variant = "info") {
    const t: Toast = {
      id: `t_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      message,
      variant,
      ts: Date.now(),
    };
    set({ toasts: [...get().toasts, t], history: [t, ...get().history].slice(0, 50) });
    setTimeout(() => get().dismiss(t.id), 4500);
  },
  dismiss(id) {
    set({ toasts: get().toasts.filter((t) => t.id !== id) });
  },
  clearHistory() {
    set({ history: [] });
  },
}));
