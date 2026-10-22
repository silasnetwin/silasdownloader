import { createContext, useContext } from "react";

import type {
  DownloadRequest,
  JobState,
  Settings,
  HistoryEntry,
  ServerStatus,
} from "../lib/types";

/** Ein Auftrag in der Queue. */
export interface Job {
  id: number;
  url: string;
  title: string;
  state: JobState;
  progress: number;
  status: string;
  speed: string;
  eta: string;
  total: string;
  outputFile: string | null;
  error: string | null;
  playlistIndex: number | null;
  /** Wie oft der Auftrag schon automatisch wiederholt wurde. */
  attempts: number;
  /** Wird benötigt, um den Auftrag erneut zu starten. */
  request: unknown;
}

export interface Toast {
  id: number;
  text: string;
  kind: "info" | "error";
}

export interface AppContextValue {
  settings: Settings;
  patchSettings: (patch: Partial<Settings>) => void;
  history: HistoryEntry[];
  setHistory: React.Dispatch<React.SetStateAction<HistoryEntry[]>>;
  jobs: Job[];
  addJob: (job: Omit<Job, "id" | "state" | "progress" | "status" | "speed" | "eta" | "total" | "outputFile" | "error">) => number;
  enqueue: (job: {
    url: string;
    title: string;
    request: DownloadRequest;
    playlistIndex?: number | null;
  }) => void;
  updateJob: (id: number, patch: Partial<Job>) => void;
  removeJob: (id: number) => void;
  retryJob: (id: number) => void;
  cancelJob: (id: number) => void;
  logs: string[];
  pushLog: (line: string) => void;
  clearLogs: () => void;
  toasts: Toast[];
  notify: (text: string, kind?: "info" | "error") => void;
  dismissToast: (id: number) => void;
  theme: "dark" | "light";
  toggleTheme: () => void;
  engine: { ytdlp: string; ffmpeg: string };
  page: string;
  setPage: (page: string) => void;
  converter: {
    input: string | null;
    output: string | null;
    progress: number;
    status: string;
    running: boolean;
  };
  setConverterInput: (path: string | null) => void;
  setConverterOutput: (path: string | null) => void;
  /** Der Zustand des Tresors mit den Zugangsdaten des Servers. */
  server: ServerStatus | null;
  /** Liest den Zustand des Tresors neu. */
  refreshServer: () => Promise<void>;
}

export const AppContext = createContext<AppContextValue | null>(null);

/** Liefert den Anwendungszustand. */
export function useApp(): AppContextValue {
  const value = useContext(AppContext);
  if (!value) throw new Error("AppContext fehlt");
  return value;
}
