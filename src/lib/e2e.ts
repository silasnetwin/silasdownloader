// Prüfstand für die Oberfläche: ersetzt die Rust-Seite durch feste Antworten.
//
// Wird nur geladen, wenn die Seite mit `?e2e=1` aufgerufen wird. Die
// eigentliche App merkt davon nichts; im fertigen Programm ist der Code nicht
// enthalten, weil `import.meta.env.DEV` dort falsch ist.

import { emit } from "@tauri-apps/api/event";
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
  closeToTray: true,
  autostart: false,
  embedLyrics: true,
  language: "de",
};

/** Ein Video, das die Analyse liefert. */
const videoPreview = {
  kind: "video" as const,
  url: "https://example.com/video/1",
  id: "video-1",
  title: "Testvideo",
  channel: "Testkanal",
  duration: 95,
  uploadDate: "20250102",
  thumbnailDataUrl: null,
  formats: [
    {
      id: "18",
      ext: "mp4",
      resolution: "640x360",
      height: 360,
      fps: 30,
      vcodec: "avc1",
      acodec: "mp4a",
      filesize: 1_048_576,
      audioOnly: false,
      videoOnly: false,
    },
  ],
  subtitleLanguages: [],
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
        case "analyze_link":
          return videoPreview;
        case "start_download":
          return (payload as { jobId: number }).jobId;
        case "engine_versions":
          return ["2025.10.22", "7.1"];
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

  // Für die Oberflächentests: Aufrufe und Engine-Ereignisse von außen.
  (
    window as unknown as {
      __silasE2E: {
        calls: typeof calls;
        emit: (event: string, payload: unknown) => Promise<void>;
      };
    }
  ).__silasE2E = {
    calls,
    emit: (event, payload) => emit(event, payload),
  };
}
