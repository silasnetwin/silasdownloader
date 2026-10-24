//! SilasDownloader — Tauri-Anwendung.

pub mod deeplink;
pub mod engine;
pub mod lyrics;
pub mod model;
pub mod spotify;
mod storage;
pub mod tray;

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use engine::process::spawn_download;
use model::{
    ConvertFormat, DownloadRequest, HistoryEntry, LinkResult, ListPreview, PlaylistPreview, Preset,
    SeriesPreview, Settings, VideoPreview,
};
use serde::Serialize;
use spotify::workflow::{SpotifyWorkflowRequest, SpotifyWorkflowResult};
use tauri::Emitter;

/// Ein Track aus der Spotify-Vorschau.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyPreviewItem {
    pub title: String,
    pub artists: String,
    pub album: String,
    pub track_number: Option<u32>,
    pub duration: Option<u64>,
    /// Das Albumcover als Bild in der Oberfläche.
    pub cover: Option<String>,
}

/// Die gelesene Spotify-Auswahl.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyPreview {
    pub title: String,
    pub tracks: usize,
    pub listed_tracks: usize,
    pub items: Vec<SpotifyPreviewItem>,
}
use tauri::{AppHandle, Manager, State};

/// Laufende Aufträge und ihre Abbruchschalter.
#[derive(Default)]
pub struct AppState {
    pub cancels: Mutex<HashMap<u64, Arc<AtomicBool>>>,
    /// Die Prozesskennungen der laufenden Engine-Prozesse.
    pub pids: Mutex<HashMap<u64, u32>>,
}

impl AppState {
    /// Merkt sich den Prozess eines laufenden Auftrags.
    pub fn remember_pid(&self, job_id: u64, pid: u32) {
        if let Ok(mut pids) = self.pids.lock() {
            pids.insert(job_id, pid);
        }
    }

    /// Vergisst einen beendeten Auftrag.
    pub fn forget_pid(&self, job_id: u64) {
        if let Ok(mut pids) = self.pids.lock() {
            pids.remove(&job_id);
        }
    }

    /// Bricht alle laufenden Downloads ab und beendet ihre Prozesse.
    ///
    /// Ohne das bleiben die Kindprozesse zurück, wenn die App geschlossen wird. Sie
    /// halten dann die Zieldateien gesperrt und laden im Hintergrund weiter.
    pub fn stop_all(&self) {
        if let Ok(cancels) = self.cancels.lock() {
            for cancel in cancels.values() {
                cancel.store(true, Ordering::Relaxed);
            }
        }
        let pids: Vec<u32> = self
            .pids
            .lock()
            .map(|map| map.values().copied().collect())
            .unwrap_or_default();
        for pid in pids {
            kill_process_tree(pid);
        }
    }
}

/// Beendet einen Prozess samt Kindprozessen.
fn kill_process_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut command = std::process::Command::new("taskkill");
        command
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        command.creation_flags(CREATE_NO_WINDOW);
        let _ = command.status();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
}

/// Analysiert ein Video.
#[tauri::command]
fn analyze_video(
    app: AppHandle,
    url: String,
    extractor_args: String,
) -> Result<VideoPreview, String> {
    engine::metadata::analyze_video(Some(&app), &url, &extractor_args)
}

/// Analysiert eine Playlist.
#[tauri::command]
fn analyze_playlist(
    app: AppHandle,
    url: String,
    extractor_args: String,
) -> Result<PlaylistPreview, String> {
    engine::metadata::analyze_playlist(Some(&app), &url, &extractor_args)
}

/// Liest eine Spotify-Auswahl und lädt jeden Titel als MP3.
#[tauri::command]
async fn spotify_workflow(
    app: AppHandle,
    request: SpotifyWorkflowRequest,
) -> Result<SpotifyWorkflowResult, String> {
    let progress_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        spotify::workflow::run_workflow(&progress_app, request, |update| {
            let _ = progress_app.emit("spotify://progress", update);
        })
    })
    .await
    .map_err(|error| format!("Der Spotify-Ablauf konnte nicht beendet werden: {error}"))?
}

/// Wie viele Cover in der Vorschau gezeigt werden.
const PREVIEW_COVERS: usize = 12;

/// Liest eine Spotify-Auswahl, ohne etwas herunterzuladen.
#[tauri::command]
async fn spotify_preview(url: String) -> Result<SpotifyPreview, String> {
    let temporary = std::env::temp_dir().join("silas-spotify-preview");
    let _ = std::fs::remove_dir_all(&temporary);
    let covers_dir = temporary.join("covers");
    let source = tauri::async_runtime::spawn_blocking(move || {
        spotify::fetch_public_source(&url, &covers_dir, PREVIEW_COVERS)
    })
    .await
    .map_err(|error| format!("Die Spotify-Auswahl konnte nicht gelesen werden: {error}"))??;
    // Die Cover als Bilddaten lesen, solange die Dateien noch da sind.
    let items: Vec<SpotifyPreviewItem> = source
        .tracks
        .iter()
        .map(|track| SpotifyPreviewItem {
            cover: track
                .artwork_path
                .as_ref()
                .and_then(|path| std::fs::read(path).ok())
                .map(|bytes| format!("data:image/jpeg;base64,{}", base64_encode(&bytes))),
            title: track.title.clone(),
            artists: track.artists.join(", "),
            album: track.album.clone(),
            track_number: track.track_number,
            duration: track.duration,
        })
        .collect();
    let _ = std::fs::remove_dir_all(&temporary);
    Ok(SpotifyPreview {
        title: source.title,
        tracks: source.tracks.len(),
        listed_tracks: source.listed_tracks,
        items,
    })
}

/// Wandelt Bytes in Base64 um.
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let value = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((value >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((value >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[((value >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(value & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Analysiert einen Link und erkennt selbst, was dahinter steckt.
///
/// Es wird kein certain Anbieter bevorzugt: der Reihe nach werden Serienseiten,
/// einzelne Videos und Playlists geprüft.
#[tauri::command]
fn analyze_link(app: AppHandle, url: String, extractor_args: String) -> Result<LinkResult, String> {
    if let Some(series) = engine::series::resolve_series(&url)? {
        return Ok(LinkResult::List(engine::series::to_list(series)));
    }
    match engine::metadata::analyze_video(Some(&app), &url, &extractor_args) {
        Ok(video) => Ok(LinkResult::Video(video)),
        Err(video_error) => {
            // Vielleicht ist es doch eine Seite mit mehreren Videos.
            match engine::metadata::analyze_playlist(Some(&app), &url, &extractor_args) {
                Ok(list) if !list.entries.is_empty() => {
                    Ok(LinkResult::List(engine::metadata::to_list(list)))
                }
                _ => Err(video_error),
            }
        }
    }
}

/// Listet alle Videos, die auf einer Seite zu finden sind.
#[tauri::command]
fn list_page(app: AppHandle, url: String, extractor_args: String) -> Result<ListPreview, String> {
    if let Some(series) = engine::series::resolve_series(&url)? {
        return Ok(engine::series::to_list(series));
    }
    let list = engine::metadata::analyze_playlist(Some(&app), &url, &extractor_args)?;
    if list.entries.is_empty() {
        return Err("Auf dieser Seite wurden keine Videos gefunden.".into());
    }
    Ok(engine::metadata::to_list(list))
}

/// Analysiert eine Serienseite und löst sie in ihre Folgen auf.
///
/// Gibt `None` zurück, wenn die Adresse keine Serienseite ist.
#[tauri::command]
fn analyze_series(url: String) -> Result<Option<SeriesPreview>, String> {
    engine::series::resolve_series(&url)
}

/// Liefert die eingebauten und die eigenen Presets.
#[tauri::command]
fn list_presets(settings: Settings) -> Vec<Preset> {
    let mut presets = Preset::builtins();
    presets.extend(settings.custom_presets);
    presets
}

/// Startet einen Download.
///
/// Die Auftragsnummer kommt aus der Oberfläche. Nur so passen die Ereignisse
/// der Engine (`download://…`) zu dem Eintrag, den die Oberfläche anzeigt —
/// auch wenn ein Auftrag nach einem Fehler erneut gestartet wird. Würde die
/// Engine selbst zählen, bekäme eine Wiederholung eine neue Nummer und die
/// Oberfläche würde deren Fortschritt nie sehen.
#[tauri::command]
fn start_download(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: u64,
    request: DownloadRequest,
) -> Result<u64, String> {
    let cancel = Arc::new(AtomicBool::new(false));
    state
        .cancels
        .lock()
        .map_err(|e| e.to_string())?
        .insert(job_id, cancel.clone());
    spawn_download(app, job_id, request, cancel);
    Ok(job_id)
}

/// Bricht einen laufenden Download ab.
#[tauri::command]
fn cancel_download(state: State<'_, AppState>, job_id: u64) -> Result<(), String> {
    if let Some(cancel) = state
        .cancels
        .lock()
        .map_err(|e| e.to_string())?
        .get(&job_id)
    {
        cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}

/// Meldet einen Auftrag als beendet zurück.
#[tauri::command]
fn forget_job(state: State<'_, AppState>, job_id: u64) -> Result<(), String> {
    state
        .cancels
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&job_id);
    Ok(())
}

/// Lädt die gespeicherten Einstellungen.
#[tauri::command]
fn load_settings() -> Settings {
    storage::load_settings()
}

/// Speichert die Einstellungen.
#[tauri::command]
fn save_settings(settings: Settings) -> Result<(), String> {
    storage::save_settings(&settings)
}

/// Lädt den Verlauf.
#[tauri::command]
fn load_history() -> Vec<HistoryEntry> {
    storage::load_history()
}

/// Speichert den Verlauf.
#[tauri::command]
fn save_history(history: Vec<HistoryEntry>) -> Result<(), String> {
    storage::save_history(&history)
}

/// Liefert die Versionen der eingebetteten Engine.
#[tauri::command]
fn engine_versions(app: AppHandle) -> (String, String) {
    engine::metadata::engine_versions(&app)
}

/// Konvertiert eine lokale Datei in ein anderes Format.
#[tauri::command]
fn convert_media(app: AppHandle, path: String, format: ConvertFormat) -> Result<String, String> {
    let input = std::path::PathBuf::from(&path);
    if !input.is_file() {
        return Err("Die Datei existiert nicht.".into());
    }
    let output_dir = input
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let result = engine::converter::convert(&app, &input, &output_dir, format)?;
    Ok(result.to_string_lossy().into_owned())
}

/// Öffnet einen Ordner im Explorer.
#[tauri::command]
fn reveal_path(path: String) -> Result<(), String> {
    let target = std::path::PathBuf::from(&path);
    if !target.exists() {
        return Err("Der Pfad existiert nicht.".into());
    }
    let mut command = std::process::Command::new("explorer.exe");
    if target.is_dir() {
        command.arg(&target);
    } else {
        command.arg(format!("/select,{}", target.display()));
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Ordner konnte nicht geöffnet werden: {e}"))
}

/// Aktualisiert yt-dlp auf die neueste Version.
#[tauri::command]
fn update_engine(app: AppHandle) -> Result<String, String> {
    let tools = engine::tools::resolve_tools(Some(&app))?;
    engine::tools::update_ytdlp(&tools.yt_dlp)
}

/// Meldet den Fortschritt an das Symbol im Infobereich.
#[tauri::command]
fn tray_progress(app: AppHandle, active: u32, total: u32, percent: f32) {
    tray::update(&app, active, total, percent);
}

/// Startet die Anwendung.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Muss das erste Plugin sein: eine zweite Instanz wird an diese hier
        // weitergereicht, damit eine Adresse nicht im Nichts landet.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            tray::show(app);
            if let Some(url) = deeplink::find_in_args(argv) {
                deeplink::emit(app, &url);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_deep_link::init())
        .manage(AppState::default())
        .setup(|app| {
            tray::create(app.handle())?;

            // Windows kennt die Adresse erst, wenn sie in der Registrierung steht.
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let _ = app.deep_link().register_all();
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        deeplink::emit(&handle, url.as_str());
                    }
                });
            }

            let args: Vec<String> = std::env::args().collect();
            // Beim Anmelden gestartet: nur im Infobereich zeigen.
            if args.iter().any(|arg| arg == "--minimized") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            // Kalter Start über eine Adresse: die Oberfläche ist erst später bereit.
            if let Some(url) = deeplink::find_in_args(&args) {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    deeplink::emit(&handle, &url);
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Im Infobereich bleiben die Downloads am Leben. Sonst werden alle
                // Kindprozesse beendet, damit keine gesperrten Dateien zurückbleiben.
                if tray::close_to_tray() {
                    api.prevent_close();
                    let _ = window.hide();
                    return;
                }
                window.state::<AppState>().stop_all();
            }
        })
        .invoke_handler(tauri::generate_handler![
            analyze_video,
            analyze_playlist,
            analyze_series,
            analyze_link,
            list_page,
            spotify_workflow,
            spotify_preview,
            convert_media,
            list_presets,
            start_download,
            cancel_download,
            forget_job,
            load_settings,
            save_settings,
            load_history,
            save_history,
            engine_versions,
            reveal_path,
            update_engine,
            tray_progress,
        ])
        .run(tauri::generate_context!())
        .expect("Fehler beim Starten von SilasDownloader");
}
