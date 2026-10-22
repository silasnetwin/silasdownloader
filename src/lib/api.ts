/** Schicht zwischen Oberfläche und Rust-Backend. */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  ConvertFormat,
  ContentKind,
  DownloadRequest,
  FinishedPayload,
  FilePayload,
  HistoryEntry,
  LinkResult,
  ListPreview,
  LogPayload,
  PlaylistPreview,
  Preset,
  ProgressPayload,
  SeriesPreview,
  ServerStatus,
  ServerUnlock,
  Settings,
  SpotifyPreview,
  SpotifyWorkflowResult,
  TitlePayload,
  VideoPreview,
} from "./types";

export const analyzeVideo = (url: string, extractorArgs: string) =>
  invoke<VideoPreview>("analyze_video", { url, extractorArgs });

export const analyzePlaylist = (url: string, extractorArgs: string) =>
  invoke<PlaylistPreview>("analyze_playlist", { url, extractorArgs });

/** Löst eine Serienseite (z. B. KiKA) in ihre Folgen auf. */
export const analyzeSeries = (url: string) =>
  invoke<SeriesPreview | null>("analyze_series", { url });

/**
 * Analysiert einen Link und erkennt selbst, was dahinter steckt:
 * ein einzelnes Video oder eine Liste von Videos.
 */
export const analyzeLink = (url: string, extractorArgs: string) =>
  invoke<LinkResult>("analyze_link", { url, extractorArgs });

/** Listet alle Videos, die auf einer Seite zu finden sind. */
export const listPage = (url: string, extractorArgs: string) =>
  invoke<ListPreview>("list_page", { url, extractorArgs });

/** Liest eine Spotify-Auswahl, ohne etwas herunterzuladen. */
export const spotifyPreview = (url: string) =>
  invoke<SpotifyPreview>("spotify_preview", { url });

/** Lädt eine Spotify-Auswahl als MP3. */
export const spotifyWorkflow = (request: {
  spotifyUrl: string;
  outputDir: string;
  audioQuality: number;
  embedCover: boolean;
  embedLyrics: boolean;
}) => invoke<SpotifyWorkflowResult>("spotify_workflow", { request });

/** Meldet den Fortschritt an das Symbol im Infobereich. */
export const trayProgress = (active: number, total: number, percent: number) =>
  invoke<void>("tray_progress", { active, total, percent });

/** Konvertiert eine lokale Datei mit FFmpeg. */
export const convertMedia = (path: string, format: ConvertFormat) =>
  invoke<string>("convert_media", { path, format });

/** Liefert die eingebauten und die eigenen Presets. */
export const listPresets = (settings: Settings) =>
  invoke<Preset[]>("list_presets", { settings });

export const startDownload = (request: DownloadRequest) =>
  invoke<number>("start_download", { request });

export const cancelDownload = (jobId: number) =>
  invoke<void>("cancel_download", { jobId });

export const forgetJob = (jobId: number) => invoke<void>("forget_job", { jobId });

export const loadSettings = () => invoke<Settings>("load_settings");

export const saveSettings = (settings: Settings) =>
  invoke<void>("save_settings", { settings });

export const loadHistory = () => invoke<HistoryEntry[]>("load_history");

export const saveHistory = (history: HistoryEntry[]) =>
  invoke<void>("save_history", { history });

export const engineVersions = () =>
  invoke<[string, string]>("engine_versions");

export const revealPath = (path: string) => invoke<void>("reveal_path", { path });

/** Zeigt den Zustand des Tresors an. */
export const serverStatus = () => invoke<ServerStatus>("server_status");

/** Legt den Tresor an oder öffnet ihn. */
export const serverUnlock = (data: ServerUnlock) =>
  invoke<ServerStatus>("server_unlock", {
    password: data.password,
    host: data.host,
    alias: data.alias,
    user: data.user,
    apiKey: data.apiKey,
    jellyfinUrl: data.jellyfinUrl,
  });

/** Sperrt den Tresor wieder. */
export const serverLock = () => invoke<void>("server_lock");

/** Prüft die Verbindung und holt die Bibliotheken. */
export const serverTest = () => invoke<ServerStatus>("server_test");

/** Legt eine Datei auf den Server. */
export const serverUpload = (path: string, kind: ContentKind) =>
  invoke<string>("server_upload", { path, kind });

/** Aktualisiert yt-dlp auf die neueste Version. */
export const updateEngine = () => invoke<string>("update_engine");

/** Meldet sich für die Ereignisse der Engine an. */
export const onProgress = (cb: (payload: ProgressPayload) => void): Promise<UnlistenFn> =>
  listen<ProgressPayload>("download://progress", (e) => cb(e.payload));

export const onLog = (cb: (payload: LogPayload) => void): Promise<UnlistenFn> =>
  listen<LogPayload>("download://log", (e) => cb(e.payload));

export const onTitle = (cb: (payload: TitlePayload) => void): Promise<UnlistenFn> =>
  listen<TitlePayload>("download://title", (e) => cb(e.payload));

export const onFile = (cb: (payload: FilePayload) => void): Promise<UnlistenFn> =>
  listen<FilePayload>("download://file", (e) => cb(e.payload));

export const onFinished = (cb: (payload: FinishedPayload) => void): Promise<UnlistenFn> =>
  listen<FinishedPayload>("download://finished", (e) => cb(e.payload));
