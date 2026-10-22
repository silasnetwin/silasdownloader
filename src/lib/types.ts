/** Typen, die dem Rust-Backend entsprechen (serde camelCase). */

export type DownloadMode = "video" | "audio";
export type VideoQuality = "best" | "p2160" | "p1440" | "p1080" | "p720" | "p480" | "smallest";
export type VideoContainer = "mp4" | "mkv" | "webm";
export type AudioFormat = "mp3" | "m4a" | "opus" | "flac" | "wav";

/** Die Sprache der Oberfläche. */
export type Language = "de" | "en";

export type JobState =
  | "pending"
  | "analyzing"
  | "downloading"
  | "postProcessing"
  | "completed"
  | "failed"
  | "cancelled";

export interface Settings {
  outputDir: string;
  mode: DownloadMode;
  videoQuality: VideoQuality;
  videoContainer: VideoContainer;
  audioFormat: AudioFormat;
  /** Bitrate für Audioformate in kbit/s. */
  audioQuality: number;
  embedThumbnail: boolean;
  embedMetadata: boolean;
  embedChapters: boolean;
  writeInfoJson: boolean;
  writeDescription: boolean;
  downloadSubtitles: boolean;
  autoSubtitles: boolean;
  subtitleLanguages: string;
  subtitleFormat: string;
  concurrentFragments: number;
  turboDownloads: boolean;
  aria2Connections: number;
  maxParallelDownloads: number;
  retries: number;
  speedLimitKib: number | null;
  faststart: boolean;
  downloadArchive: boolean;
  separateFolderPerVideo: boolean;
  videoFolderTemplate: string;
  filenameTemplate: string;
  filenameMaxLen: number;
  extractorArgs: string;
  extraYtdlpArgs: string;
  playlistSingleRun: boolean;
  saveHistory: boolean;
  duplicateWarning: boolean;
  openFolderOnComplete: boolean;
  theme: string;
  startPage: string;
  customPresets: Preset[];
  /** Fertige Dateien nach dem Download auf den Server legen. */
  serverUpload: boolean;
  /** Schließen legt die App in den Infobereich statt sie zu beenden. */
  closeToTray: boolean;
  /** Beim Anmelden von Windows starten (im Hintergrund). */
  autostart: boolean;
  /** Songtexte beim Spotify-Download holen und einbetten. */
  embedLyrics: boolean;
  /** Die Sprache der Oberfläche. */
  language: Language;
}

export interface MediaFormat {
  id: string;
  ext: string;
  resolution: string;
  height: number | null;
  fps: number | null;
  vcodec: string;
  acodec: string;
  filesize: number | null;
  audioOnly: boolean;
  videoOnly: boolean;
}

export interface VideoPreview {
  url: string;
  id: string;
  title: string;
  channel: string;
  duration: number | null;
  uploadDate: string;
  thumbnailDataUrl: string | null;
  formats: MediaFormat[];
  subtitleLanguages: string[];
}

export interface PlaylistEntry {
  index: number;
  id: string;
  url: string;
  title: string;
  duration: number | null;
  selected: boolean;
}

export interface PlaylistPreview {
  url: string;
  title: string;
  entries: PlaylistEntry[];
}

export interface PlaylistBatch {
  playlistUrl: string;
  itemsSpec: string;
  indices: number[];
  total: number;
}

/** Eine einzelne Folge einer Videoreihe. */
export interface SeriesEpisode {
  index: number;
  title: string;
  url: string;
  duration: number | null;
  season: number | null;
  episode: number | null;
}

/** Das Ergebnis einer aufgelösten Serienseite. */
export interface SeriesPreview {
  url: string;
  title: string;
  seasons: number[];
  episodes: SeriesEpisode[];
}

/** Ein Eintrag in einer Liste von Videos. */
export interface ListEntry {
  index: number;
  title: string;
  url: string;
  duration: number | null;
  season: number | null;
  episode: number | null;
  selected: boolean;
}

/** Das Ergebnis einer Seiten-Analyse: mehrere Videos auf einer Seite. */
export interface ListPreview {
  url: string;
  title: string;
  listKind: "playlist" | "series";
  seasons: number[];
  entries: ListEntry[];
}

/** Das Ergebnis einer Link-Analyse. */
export type LinkResult =
  | ({ kind: "video" } & VideoPreview)
  | ({ kind: "list" } & ListPreview);

/** Ein Track aus der Spotify-Vorschau. */
export interface SpotifyPreviewItem {
  title: string;
  artists: string;
  album: string;
  trackNumber: number | null;
  duration: number | null;
  /** Das Albumcover als Bild in der Oberfläche. */
  cover: string | null;
}

/** Die gelesene Spotify-Auswahl. */
export interface SpotifyPreview {
  title: string;
  tracks: number;
  listedTracks: number;
  items: SpotifyPreviewItem[];
}

/** Der Fortschritt eines einzelnen Spotify-Tracks. */
export interface SpotifyProgress {
  index: number;
  total: number;
  title: string;
  percent: number | null;
  status: string;
  completed: boolean;
  error: string | null;
  outputFile: string | null;
}

/** Das Ergebnis des Spotify-Ablaufs. */
export interface SpotifyWorkflowResult {
  playlistTitle: string;
  tracks: number;
  listedTracks: number;
  downloaded: number;
  failed: number;
  outputDir: string;
}

/** Ein gespeichertes Download-Preset. */
export interface Preset {
  name: string;
  mode: DownloadMode;
  videoQuality: VideoQuality;
  videoContainer: VideoContainer;
  audioFormat: AudioFormat;
  embedMetadata: boolean;
  embedThumbnail: boolean;
  subtitles: boolean;
}

export type ConvertFormat =
  | "mp4"
  | "mkv"
  | "webm"
  | "mov"
  | "mp3"
  | "m4a"
  | "wav"
  | "flac";

export interface DownloadRequest {
  url: string;
  settings: Settings;
  playlistIndex: number | null;
  playlistBatch: PlaylistBatch | null;
  selectedVideoFormat: string | null;
  selectedAudioFormat: string | null;
  /** Was für ein Inhalt geladen wird. Entscheidet die Zielbibliothek auf dem Server. */
  kind: ContentKind;
}

export interface HistoryEntry {
  id: number;
  url: string;
  title: string;
  downloadedAt: string;
  format: string;
  quality: string;
  outputFile: string;
  success: boolean;
}

export interface ProgressPayload {
  jobId: number;
  value: number;
  state: JobState;
  status: string;
  speed: string;
  eta: string;
  total: string;
}

export interface LogPayload {
  jobId: number;
  level: "info" | "warning" | "error" | "debug";
  line: string;
}

export interface TitlePayload {
  jobId: number;
  title: string;
  playlistIndex: number | null;
}

export interface FilePayload {
  jobId: number;
  path: string;
}

export interface FinishedPayload {
  jobId: number;
  ok: boolean;
  error: string | null;
  technicalDetails: string;
  outputFile: string | null;
  outputFiles: string[];
}

/** Eine Bibliothek von Jellyfin mit ihrem Zielordner auf dem Server. */
export interface ServerLibrary {
  name: string;
  /** Die Art der Sammlung: movies, tvshows oder playlists. */
  collectionType: string;
  path: string;
}

/** Die Art des geladenen Inhalts. */
export type ContentKind = "movie" | "series" | "music";

/** Der Zustand des Tresors mit den Zugangsdaten des Servers. */
export interface ServerStatus {
  configured: boolean;
  unlocked: boolean;
  host: string;
  alias: string;
  user: string;
  jellyfinUrl: string;
  libraries: ServerLibrary[];
  /** Die Geräte im Tailscale-Netz. */
  peers: string[];
  error: string | null;
}

/** Die geforderten Angaben zum Entsperren des Tresors. */
export interface ServerUnlock {
  password: string;
  host: string;
  alias: string;
  user: string;
  apiKey: string;
  jellyfinUrl: string;
}
