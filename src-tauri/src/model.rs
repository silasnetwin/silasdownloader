use std::{collections::BTreeSet, path::PathBuf};

use serde::{Deserialize, Serialize};

/// Modus eines Downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadMode {
    Video,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VideoQuality {
    Best,
    P2160,
    P1440,
    P1080,
    P720,
    P480,
    Smallest,
}

impl VideoQuality {
    pub fn max_height(self) -> Option<u32> {
        match self {
            Self::P2160 => Some(2160),
            Self::P1440 => Some(1440),
            Self::P1080 => Some(1080),
            Self::P720 => Some(720),
            Self::P480 => Some(480),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VideoContainer {
    Mp4,
    Mkv,
    Webm,
}

impl VideoContainer {
    pub fn yt_name(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
            Self::Webm => "webm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioFormat {
    Mp3,
    M4a,
    Opus,
    Flac,
    Wav,
}

impl AudioFormat {
    pub fn yt_name(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Opus => "opus",
            Self::Flac => "flac",
            Self::Wav => "wav",
        }
    }
}

/// Ein Eintrag in einer Liste von Videos.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEntry {
    pub index: u32,
    pub title: String,
    pub url: String,
    pub duration: Option<u64>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub selected: bool,
}

/// Das Ergebnis einer Seiten-Analyse: mehrere Videos auf einer Seite.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPreview {
    pub url: String,
    pub title: String,
    /// `playlist` oder `series` – beeinflusst nur die Beschriftung.
    pub list_kind: String,
    pub seasons: Vec<u32>,
    pub entries: Vec<ListEntry>,
}

/// Das Ergebnis einer Link-Analyse: entweder ein einzelnes Video oder eine Liste.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LinkResult {
    Video(VideoPreview),
    List(ListPreview),
}

/// Zusätzliche Metadaten, die nach dem Download in die Datei geschrieben werden.
///
/// Wird für Spotify-Tracks benutzt: dort stammen Album, Album-Interpret,
/// Tracknummer und Cover aus der Spotify-Seite und nicht aus dem Video.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub track_number: Option<u32>,
    /// Erscheinungsjahr.
    pub year: Option<u32>,
    pub artwork_path: Option<String>,
    /// Songtext, wird als USLT in die Datei geschrieben.
    #[serde(default)]
    pub lyrics: Option<String>,
}

/// Ein gespeichertes Download-Preset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Preset {
    pub name: String,
    pub mode: DownloadMode,
    pub video_quality: VideoQuality,
    pub video_container: VideoContainer,
    pub audio_format: AudioFormat,
    pub embed_metadata: bool,
    pub embed_thumbnail: bool,
    pub subtitles: bool,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            name: "Standard".into(),
            mode: DownloadMode::Video,
            video_quality: VideoQuality::P1080,
            video_container: VideoContainer::Mp4,
            audio_format: AudioFormat::Mp3,
            embed_metadata: true,
            embed_thumbnail: true,
            subtitles: false,
        }
    }
}

impl Preset {
    /// Die eingebauten Voreinstellungen.
    pub fn builtins() -> Vec<Self> {
        vec![
            Self {
                name: "Standard".into(),
                ..Default::default()
            },
            Self {
                name: "Beste Qualität".into(),
                video_quality: VideoQuality::Best,
                video_container: VideoContainer::Mkv,
                ..Default::default()
            },
            Self {
                name: "Musik".into(),
                mode: DownloadMode::Audio,
                video_quality: VideoQuality::Best,
                embed_thumbnail: true,
                ..Default::default()
            },
            Self {
                name: "Kleine Datei".into(),
                video_quality: VideoQuality::P720,
                video_container: VideoContainer::Mp4,
                embed_thumbnail: false,
                ..Default::default()
            },
            Self {
                name: "Archiv".into(),
                video_quality: VideoQuality::Best,
                video_container: VideoContainer::Mkv,
                subtitles: true,
                ..Default::default()
            },
        ]
    }
}

/// Eine einzelne Folge einer Videoreihe.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesEpisode {
    pub index: u32,
    pub title: String,
    pub url: String,
    pub duration: Option<u64>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
}

/// Das Ergebnis einer aufgelösten Serienseite.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesPreview {
    pub url: String,
    pub title: String,
    pub seasons: Vec<u32>,
    pub episodes: Vec<SeriesEpisode>,
}

/// Zielformat des Media-Konverters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConvertFormat {
    Mp4,
    Mkv,
    Webm,
    Mov,
    Mp3,
    M4a,
    Wav,
    Flac,
}

impl ConvertFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Mp4 => "MP4",
            Self::Mkv => "MKV",
            Self::Webm => "WebM",
            Self::Mov => "MOV",
            Self::Mp3 => "MP3",
            Self::M4a => "M4A",
            Self::Wav => "WAV",
            Self::Flac => "FLAC",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
            Self::Webm => "webm",
            Self::Mov => "mov",
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Wav => "wav",
            Self::Flac => "flac",
        }
    }
}

/// Alle Einstellungen der App.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub output_dir: String,
    pub mode: DownloadMode,
    pub video_quality: VideoQuality,
    pub video_container: VideoContainer,
    pub audio_format: AudioFormat,
    /// Bitrate fuer Audioformate in kbit/s.
    pub audio_quality: u32,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub embed_chapters: bool,
    pub write_info_json: bool,
    pub write_description: bool,
    pub download_subtitles: bool,
    pub auto_subtitles: bool,
    pub subtitle_languages: String,
    pub subtitle_format: String,
    pub concurrent_fragments: u32,
    pub turbo_downloads: bool,
    pub aria2_connections: u32,
    pub max_parallel_downloads: usize,
    pub retries: u32,
    pub speed_limit_kib: Option<u64>,
    pub faststart: bool,
    pub download_archive: bool,
    pub separate_folder_per_video: bool,
    pub video_folder_template: String,
    pub filename_template: String,
    pub filename_max_len: u32,
    pub extractor_args: String,
    pub extra_ytdlp_args: String,
    pub playlist_single_run: bool,
    pub save_history: bool,
    pub duplicate_warning: bool,
    pub open_folder_on_complete: bool,
    /// Dunkles oder helles Design.
    pub theme: String,
    /// Seite, die beim Start gezeigt wird.
    pub start_page: String,
    /// Eigene Presets des Nutzers.
    pub custom_presets: Vec<Preset>,
    /// Fertige Dateien nach dem Download auf den Server legen.
    ///
    /// Das wirkt nur, wenn der Tresor geöffnet ist. Ohne Passwort bleibt der
    /// Server-Upload abgeschaltet.
    pub server_upload: bool,
    /// Schließen legt die App in den Infobereich statt sie zu beenden.
    pub close_to_tray: bool,
    /// Beim Anmelden von Windows starten.
    pub autostart: bool,
    /// Songtexte beim Spotify-Download holen und einbetten.
    pub embed_lyrics: bool,
    /// Die Sprache der Oberfläche: `de` oder `en`.
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        let downloads = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("SilasDownloader");
        Self {
            output_dir: downloads.to_string_lossy().into_owned(),
            mode: DownloadMode::Video,
            video_quality: VideoQuality::P1080,
            video_container: VideoContainer::Mp4,
            audio_format: AudioFormat::Mp3,
            audio_quality: 320,
            embed_thumbnail: true,
            embed_metadata: true,
            embed_chapters: true,
            write_info_json: false,
            write_description: false,
            download_subtitles: false,
            auto_subtitles: false,
            subtitle_languages: "de,en".into(),
            subtitle_format: "srt".into(),
            concurrent_fragments: 12,
            turbo_downloads: true,
            aria2_connections: 8,
            max_parallel_downloads: 6,
            retries: 10,
            speed_limit_kib: None,
            faststart: true,
            download_archive: false,
            separate_folder_per_video: true,
            video_folder_template: "{title} [{id}]".into(),
            filename_template: "{title} [{id}]".into(),
            filename_max_len: 180,
            extractor_args: String::new(),
            extra_ytdlp_args: String::new(),
            playlist_single_run: false,
            save_history: true,
            duplicate_warning: true,
            open_folder_on_complete: false,
            theme: "dark".into(),
            start_page: "home".into(),
            custom_presets: Vec::new(),
            server_upload: false,
            close_to_tray: true,
            autostart: false,
            embed_lyrics: true,
            language: "de".into(),
        }
    }
}

/// Ein konkretes Format eines Videos.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaFormat {
    pub id: String,
    pub ext: String,
    pub resolution: String,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub vcodec: String,
    pub acodec: String,
    pub filesize: Option<u64>,
    pub audio_only: bool,
    pub video_only: bool,
}

/// Analyse-Ergebnis eines Videos.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPreview {
    pub url: String,
    pub id: String,
    pub title: String,
    pub channel: String,
    pub duration: Option<u64>,
    pub upload_date: String,
    pub thumbnail_data_url: Option<String>,
    pub formats: Vec<MediaFormat>,
    pub subtitle_languages: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntry {
    pub index: u32,
    pub id: String,
    pub url: String,
    pub title: String,
    pub duration: Option<u64>,
    pub selected: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistPreview {
    pub url: String,
    pub title: String,
    pub entries: Vec<PlaylistEntry>,
}

/// Zustand eines Queue-Eintrags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobState {
    Pending,
    Analyzing,
    Downloading,
    PostProcessing,
    Completed,
    Failed,
    Cancelled,
}

/// Die Art des Inhalts.
///
/// Sie entscheidet, in welche Bibliothek von Jellyfin eine fertige Datei gehört.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    /// Ein einzelner Film.
    #[default]
    Movie,
    /// Eine Serie mit mehreren Folgen.
    Series,
    /// Musik aus einem Streamingdienst.
    Music,
}

impl ContentKind {
    /// Die Art der Jellyfin-Sammlung, in die der Inhalt gehört.
    pub fn collection_type(self) -> &'static str {
        match self {
            ContentKind::Movie => "movies",
            ContentKind::Series => "tvshows",
            // Musik hat in Jellyfin keine eigene Sammlung über die Ordner.
            ContentKind::Music => "",
        }
    }

    /// Ob der Inhalt auf den Server gelegt werden soll.
    ///
    /// Musik bleibt lokal — sie gehört in die eigene Mediathek, nicht in die
    /// Filmsammlung des Servers.
    pub fn goes_to_server(self) -> bool {
        !matches!(self, ContentKind::Music)
    }
}

/// Ein Auftrag, den das Frontend an das Backend meldet.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub url: String,
    pub settings: Settings,
    #[serde(default)]
    pub playlist_index: Option<u32>,
    /// Wenn gesetzt, wird die ganze Playlist in einem Durchgang geladen.
    #[serde(default)]
    pub playlist_batch: Option<PlaylistBatch>,
    #[serde(default)]
    pub selected_video_format: Option<String>,
    #[serde(default)]
    pub selected_audio_format: Option<String>,
    /// Zusätzliche Metadaten, die nach dem Download gesetzt werden.
    #[serde(default)]
    pub metadata: Option<MediaMetadata>,
    /// Was für ein Inhalt geladen wird.
    #[serde(default)]
    pub kind: ContentKind,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistBatch {
    pub playlist_url: String,
    pub items_spec: String,
    pub indices: Vec<u32>,
    pub total: u32,
}

/// Ein Eintrag im Download-Verlauf.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub downloaded_at: chrono::DateTime<chrono::Utc>,
    pub format: String,
    pub quality: String,
    pub output_file: String,
    pub success: bool,
}

/// Sammelt alle verfügbaren Untertitelsprachen.
pub fn collect_language_codes(value: &serde_json::Value) -> Vec<String> {
    let mut languages = BTreeSet::new();
    for key in ["subtitles", "automatic_captions"] {
        if let Some(obj) = value.get(key).and_then(serde_json::Value::as_object) {
            for k in obj.keys() {
                languages.insert(k.clone());
            }
        }
    }
    languages.into_iter().collect()
}

/// Baut eine Auswahl wie `1-3,5,7-9` für die Engine.
pub fn playlist_items_spec(indices: &[u32]) -> String {
    let mut sorted = indices.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut parts: Vec<String> = Vec::new();
    let mut start: Option<u32> = None;
    let mut previous: Option<u32> = None;
    for index in sorted {
        match (start, previous) {
            (Some(_), Some(p)) if index == p + 1 => previous = Some(index),
            (Some(s), _) => {
                let end = previous.unwrap_or(s);
                parts.push(if s == end {
                    s.to_string()
                } else {
                    format!("{s}-{end}")
                });
                start = Some(index);
                previous = Some(index);
            }
            (None, _) => {
                start = Some(index);
                previous = Some(index);
            }
        }
    }
    if let (Some(s), Some(p)) = (start, previous) {
        parts.push(if s == p {
            s.to_string()
        } else {
            format!("{s}-{p}")
        });
    }
    parts.join(",")
}
