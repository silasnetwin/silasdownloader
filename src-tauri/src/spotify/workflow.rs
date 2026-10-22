//! Der Spotify-Ablauf: öffentliche Auswahl lesen, Titel suchen, als MP3 laden.
//!
//! Die Logik stammt aus `raubkopien/spotify-metadata-exporter` und ist auf die
//! Engine dieser App umgestellt. Die Aufträge laufen im Kennungsbereich ab
//! `JOB_ID_BASE`, damit sie sich von den übrigen Downloads unterscheiden lassen.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{
        atomic::AtomicBool,
        mpsc, Arc, Mutex,
    },
    thread,
};

use serde::Serialize;
use tauri::{AppHandle, Listener};

use crate::{
    engine::{
        naming::safe_filename,
        process::{run_download, FinishedPayload, ProgressPayload},
    },
    model::DownloadRequest,
    spotify::{
        audio_settings, fetch_public_source, parallel_jobs, search_query, to_metadata, SpotifyTrack,
    },
};

/// Der Kennungsbereich für die Aufträge dieses Ablaufs.
///
/// Die normalen Downloads zählen ab 1. So überschneiden sich die Ereignisse nicht.
const JOB_ID_BASE: u64 = 1_000_000;

/// Die Ereignisse der Engine, die dieser Ablauf braucht.
enum EngineEvent {
    Progress(ProgressPayload),
    Finished(FinishedPayload),
}

/// Die Anfrage für den Spotify-Ablauf.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyWorkflowRequest {
    pub spotify_url: String,
    pub output_dir: String,
    pub audio_quality: u32,
    pub embed_cover: bool,
    /// Songtexte holen und als USLT einbetten.
    #[serde(default)]
    pub embed_lyrics: bool,
}

/// Das Ergebnis des Spotify-Ablaufs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyWorkflowResult {
    pub playlist_title: String,
    pub tracks: usize,
    pub listed_tracks: usize,
    pub downloaded: usize,
    pub failed: usize,
    pub output_dir: String,
}

/// Der Fortschritt eines einzelnen Tracks.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyProgress {
    pub index: usize,
    pub total: usize,
    pub title: String,
    pub percent: Option<f32>,
    pub status: String,
    pub completed: bool,
    pub error: Option<String>,
    pub output_file: Option<String>,
}

/// Liest die Auswahl und lädt jeden Titel als MP3.
pub fn run_workflow(
    app: &AppHandle,
    request: SpotifyWorkflowRequest,
    on_progress: impl Fn(SpotifyProgress),
) -> Result<SpotifyWorkflowResult, String> {
    let spotify_url = request.spotify_url.trim().to_owned();
    let output_dir = PathBuf::from(request.output_dir.trim());
    if spotify_url.is_empty() || output_dir.as_os_str().is_empty() {
        return Err("Spotify-Link und Zielordner sind erforderlich.".into());
    }
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Zielordner konnte nicht erstellt werden: {e}"))?;

    // Cover und Zwischendateien liegen in einem temporären Ordner, damit der gewählte
    // Zielordner nur die MP3-Dateien enthält.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or_default();
    let temporary = std::env::temp_dir().join(format!("silas-spotify-{stamp}"));
    let _ = std::fs::remove_dir_all(&temporary);
    std::fs::create_dir_all(&temporary)
        .map_err(|e| format!("Temporärer Arbeitsordner konnte nicht erstellt werden: {e}"))?;

    // Die Cover landen direkt im Zielordner, also neben den MP3-Dateien. Sie gehen
    // damit nicht verloren, wenn der temporäre Arbeitsordner verschwindet.
    // Ist das Einbetten ausgeschaltet, werden gar keine Cover geholt.
    let cover_limit = if request.embed_cover { usize::MAX } else { 0 };
    let source = fetch_public_source(&spotify_url, &output_dir, cover_limit)?;
    let tracks: Vec<SpotifyTrack> = source.tracks.clone();
    let result = download_tracks(&app, tracks, &output_dir, &request, &on_progress);
    let _ = std::fs::remove_dir_all(&temporary);
    result.map(|summary| SpotifyWorkflowResult {
        playlist_title: source.title,
        tracks: source.tracks.len(),
        listed_tracks: source.listed_tracks,
        downloaded: summary.0,
        failed: summary.1,
        output_dir: output_dir.to_string_lossy().into_owned(),
    })
}

/// Sucht und lädt jeden Track, vier parallel.
fn download_tracks(
    app: &AppHandle,
    tracks: Vec<SpotifyTrack>,
    output_dir: &std::path::Path,
    request: &SpotifyWorkflowRequest,
    on_progress: &impl Fn(SpotifyProgress),
) -> Result<(usize, usize), String> {
    let mut seen = HashSet::new();
    let jobs: Vec<(usize, SpotifyTrack)> = tracks
        .into_iter()
        .filter(|track| {
            let key = if track.spotify_id.is_empty() {
                format!(
                    "{}\u{0}{}",
                    track.title.to_lowercase(),
                    track.artists.join(" ").to_lowercase()
                )
            } else {
                track.spotify_id.clone()
            };
            seen.insert(key)
        })
        .enumerate()
        .map(|(index, track)| (index + 1, track))
        .collect();

    let total = jobs.len();
    if total == 0 {
        return Ok((0, 0));
    }

    // Die Titel der Aufträge merken, damit die Ereignisse zugeordnet werden können.
    let titles: Arc<Mutex<HashMap<u64, String>>> = Arc::new(Mutex::new(HashMap::new()));
    for (index, track) in &jobs {
        let _ = titles.lock().ok().map(|mut map| {
            map.insert(JOB_ID_BASE + *index as u64, track.title.clone());
        });
    }

    // Die Ereignisse der Engine abhören. Der Kennungsbereich trennt sie von den
    // übrigen Downloads.
    let (tx, rx) = mpsc::channel::<EngineEvent>();
    let progress_tx = tx.clone();
    let progress_listener = app.listen("download://progress", move |event| {
        if let Ok(payload) = serde_json::from_str::<ProgressPayload>(event.payload()) {
            if payload.job_id >= JOB_ID_BASE {
                let _ = progress_tx.send(EngineEvent::Progress(payload));
            }
        }
    });
    let finished_tx = tx.clone();
    let finished_listener = app.listen("download://finished", move |event| {
        if let Ok(payload) = serde_json::from_str::<FinishedPayload>(event.payload()) {
            if payload.job_id >= JOB_ID_BASE {
                let _ = finished_tx.send(EngineEvent::Finished(payload));
            }
        }
    });
    drop(tx);

    let queue = Arc::new(Mutex::new(VecDeque::from(jobs)));
    let worker_count = parallel_jobs(total);
    let mut workers = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let queue = Arc::clone(&queue);
        let output = output_dir.to_owned();
        let app = app.clone();
        let quality = request.audio_quality;
        let embed_cover = request.embed_cover;
        let embed_lyrics = request.embed_lyrics;
        let handle = thread::spawn(move || loop {
            let job = queue.lock().ok().and_then(|mut guard| guard.pop_front());
            let Some((index, track)) = job else { break };
            let job_id = JOB_ID_BASE + index as u64;
            let query = search_query(&track.title, &track.artists);
            let mut settings = audio_settings(&output, quality, embed_cover);
            settings.filename_template = safe_filename(&track.title);
            let mut metadata = to_metadata(&track);
            // Der Songtext wird vor dem Download geholt: danach schreibt ihn die
            // Engine zusammen mit den übrigen Tags in die Datei.
            if embed_lyrics {
                metadata.lyrics = crate::lyrics::fetch(
                    &metadata.artist,
                    &metadata.title,
                    &metadata.album,
                    track.duration,
                );
            }
            let cancel = Arc::new(AtomicBool::new(false));
            // Der Auftrag wird blockierend ausgeführt. Erst danach holt der Worker
            // den nächsten Titel - so bleiben höchstens vier Downloads gleichzeitig aktiv.
            let _ = run_download(
                app.clone(),
                job_id,
                DownloadRequest {
                    url: format!("ytsearch1:{query}"),
                    settings,
                    playlist_index: None,
                    playlist_batch: None,
                    selected_video_format: None,
                    selected_audio_format: None,
                    metadata: Some(metadata),
                    // Musik bleibt lokal und landet nicht in der Filmsammlung.
                    kind: crate::model::ContentKind::Music,
                },
                cancel,
            );
        });
        workers.push(handle);
    }

    // Fortschritt und Abschluss zusammenführen.
    let mut downloaded = 0usize;
    let mut failed = 0usize;
    let mut finished_count = 0usize;
    while let Ok(event) = rx.recv() {
        match event {
            EngineEvent::Progress(payload) => {
                let index = (payload.job_id - JOB_ID_BASE) as usize;
                let title = titles
                    .lock()
                    .ok()
                    .and_then(|map| map.get(&payload.job_id).cloned())
                    .unwrap_or_else(|| format!("Track {index}"));
                on_progress(SpotifyProgress {
                    index,
                    total,
                    title,
                    percent: Some(payload.value * 100.0),
                    status: if payload.speed.is_empty() {
                        payload.status
                    } else {
                        format!("{} · {} · ETA {}", payload.status, payload.speed, payload.eta)
                    },
                    completed: false,
                    error: None,
                    output_file: None,
                });
            }
            EngineEvent::Finished(payload) => {
                let index = (payload.job_id - JOB_ID_BASE) as usize;
                finished_count += 1;
                let title = titles
                    .lock()
                    .ok()
                    .and_then(|map| map.get(&payload.job_id).cloned())
                    .unwrap_or_else(|| format!("Track {index}"));
                if payload.ok {
                    downloaded += 1;
                } else {
                    failed += 1;
                }
                on_progress(SpotifyProgress {
                    index,
                    total,
                    title,
                    percent: if payload.ok { Some(100.0) } else { None },
                    status: if payload.ok {
                        "MP3 fertig".into()
                    } else {
                        "Track fehlgeschlagen".into()
                    },
                    completed: true,
                    error: payload.error,
                    output_file: payload.output_file,
                });
                if finished_count >= total {
                    break;
                }
            }
        }
    }
    let _ = progress_listener;
    let _ = finished_listener;
    for worker in workers {
        if worker.join().is_err() {
            failed += 1;
        }
    }
    Ok((downloaded, failed))
}

