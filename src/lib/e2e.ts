// Prüfstand für die Oberfläche: ersetzt die Rust-Seite durch feste Antworten.
//
// Wird nur geladen, wenn die Seite mit `?e2e=1` aufgerufen wird. Die
// eigentliche App merkt davon nichts; im fertigen Programm ist der Code nicht
// enthalten, weil `import.meta.env.DEV` dort falsch ist.

import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

/** Die Einstellungen, mit denen die Oberfläche geprüft wird. */
const settings = {
  outputDir: "C:\\Downloads\\SilasDownloader",
  mode: "video",
  videoQuality: "p1080",
  videoContainer: "mp4",
  audioFormat: "mp3",
  audioQuality: 320,
  embedThumbnail: true,
  embedMetadata: true,
  embedChapters: true,
  writeInfoJson: false,
  writeDescription: false,
  downloadSubtitles: false,
  autoSubtitles: false,
  subtitleLanguages: "de,en",
  subtitleFormat: "srt",
  concurrentFragments: 12,
  turboDownloads: true,
  aria2Connections: 8,
  maxParallelDownloads: 6,
  retries: 10,
  speedLimitKib: null,
  faststart: true,
  downloadArchive: false,
  separateFolderPerVideo: true,
  videoFolderTemplate: "{title} [{id}]",
  filenameTemplate: "{title} [{id}]",
  filenameMaxLen: 180,
  extractorArgs: "",
  extraYtdlpArgs: "",
  playlistSingleRun: false,
  saveHistory: true,
  duplicateWarning: true,
  openFolderOnComplete: false,
  theme: "dark",
  startPage: "home",
  customPresets: [],
  serverUpload: false,
  closeToTray: true,
  autostart: false,
  embedLyrics: true,
  language: "de",
};

const serverStatus = {
  configured: false,
  unlocked: false,
  host: "",
  alias: "",
  user: "root",
  jellyfinUrl: "",
  libraries: [],
  peers: [],
  error: null,
};

/** Zählt die Aufrufe, damit die Tests sie prüfen können. */
export const calls: Array<{ command: string; args: unknown }> = [];

export function install() {
  mockWindows("main");
  mockIPC(
    (command, payload) => {
      calls.push({ command, args: payload });
      switch (command) {
        case "load_settings":
          return settings;
        case "save_settings":
          return null;
        case "load_history":
        case "save_history":
          return [];
        case "list_presets":
          return [];
        case "engine_versions":
          return ["2025.10.22", "7.1"];
        case "server_status":
          return serverStatus;
        case "server_unlock":
          return { ...serverStatus, configured: true, unlocked: true };
        case "server_lock":
          return null;
        case "server_test":
          return { ...serverStatus, configured: true, unlocked: true };
        case "tray_progress":
          return null;
        case "plugin:app|version":
          return "1.3.0";
        case "plugin:event|listen":
          return 1;
        case "plugin:event|unlisten":
          return null;
        case "plugin:autostart|is_enabled":
          return false;
        case "plugin:autostart|enable":
        case "plugin:autostart|disable":
          return null;
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
