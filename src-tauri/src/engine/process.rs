//! Die Download-Engine.
//!
//! Die Engine schreibt Fortschritt und Statuszeilen nach **stdout** und nur echte
//! Fehler nach stderr. Deshalb werden beide Kanäle gelesen.

use std::{
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::{
    engine::{
        metadata::friendly_error,
        tools::{hidden_command, resolve_tools, EngineTools},
    },
    model::{
        AudioFormat, DownloadMode, DownloadRequest, JobState, MediaMetadata, Settings,
        VideoContainer, VideoQuality,
    },
};

/// Protokoll zwischen Engine und Oberfläche.
///
/// Byte-basierte Felder, damit sie auch mit dem externen aria2c-Downloader gefüllt sind.
const PROGRESS_TEMPLATE: &str = "download:PROGRESS:%(progress.status)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s|%(info.playlist_index)s";

const ARIA2_ARGS: &str = "aria2c:-x {n} -s {n} -k 1M --file-allocation=none --summary-interval=0 --console-log-level=warn --no-conf=true --max-concurrent-downloads=1 --max-tries=3 --retry-wait=1 --connect-timeout=15";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub job_id: u64,
    pub value: f32,
    pub state: JobState,
    pub status: String,
    pub speed: String,
    pub eta: String,
    pub total: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogPayload {
    pub job_id: u64,
    pub level: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitlePayload {
    pub job_id: u64,
    pub title: String,
    pub playlist_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePayload {
    pub job_id: u64,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishedPayload {
    pub job_id: u64,
    pub ok: bool,
    pub error: Option<String>,
    pub technical_details: String,
    pub output_file: Option<String>,
    pub output_files: Vec<String>,
}

/// Verfolgt den Byte-Fortschritt der laufenden Streams.
///
/// Video und Audio werden getrennt geladen und danach zusammengeführt. Jedes Mal,
/// wenn der Zähler zurückspringt, beginnt ein neuer Stream. Dadurch springt der
/// Fortschritt nicht auf null zurück.
#[derive(Debug, Default)]
struct ProgressTracker {
    finished_bytes: u64,
    finished_total: u64,
    current_bytes: u64,
    current_total: u64,
    has_current: bool,
    playlist_index: Option<u32>,
    reported: f32,
}

impl ProgressTracker {
    fn push(&mut self, downloaded: Option<u64>, total: Option<u64>) {
        let Some(downloaded) = downloaded else { return };
        if self.has_current && downloaded < self.current_bytes {
            self.finished_bytes += self.current_bytes.max(self.current_total);
            self.finished_total += self.current_total;
            self.current_bytes = 0;
            self.current_total = 0;
            self.has_current = false;
        }
        self.has_current = true;
        self.current_bytes = downloaded;
        if let Some(total) = total {
            if total > 0 {
                self.current_total = total;
            }
        }
    }

    /// Vergisst die Byte-Zähler, wenn das nächste Video einer Playlist beginnt.
    fn reset_streams(&mut self) {
        self.finished_bytes = 0;
        self.finished_total = 0;
        self.current_bytes = 0;
        self.current_total = 0;
        self.has_current = false;
    }

    fn fraction(&self) -> Option<f32> {
        let total = self.finished_total + self.current_total;
        if total == 0 {
            return None;
        }
        let done = self.finished_bytes + self.current_bytes;
        Some(((done as f64 / total as f64).clamp(0.0, 1.0)) as f32)
    }

    /// Der Fortschritt bewegt sich ausschließlich nach vorne.
    fn report(&mut self, value: f32) -> f32 {
        self.reported = self.reported.max(value.clamp(0.0, 1.0));
        self.reported
    }
}

/// Bildet den Fortschritt der aktuellen Datei auf den ganzen Auftrag ab.
fn overall_value(
    tracker: &ProgressTracker,
    batch_total: u32,
    batch_indices: &[u32],
    file_fraction: f32,
) -> f32 {
    if batch_total > 1 {
        let original = tracker.playlist_index.unwrap_or(1);
        let position = batch_indices
            .iter()
            .position(|i| *i == original)
            .map(|p| p as u32 + 1)
            .unwrap_or(original.max(1));
        (((position - 1) as f32 + file_fraction) / batch_total as f32).clamp(0.0, 1.0)
    } else {
        file_fraction.clamp(0.0, 1.0)
    }
}

/// Startet einen Download im Hintergrund.
///
/// Die Ereignisse gehen an die Oberfläche. Der Spotify-Ablauf hört selbst auf die
/// Ereignisse und erkennt seine Aufträge an deren Kennung.
pub fn spawn_download<R: Runtime>(
    app: AppHandle<R>,
    job_id: u64,
    request: DownloadRequest,
    cancel: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let _ = run_download(app, job_id, request, cancel);
    });
}

/// Führt einen Download aus und wartet, bis er fertig ist.
///
/// Das ist die Grundlage für Abläufe mit begrenzter Parallelität: ein Aufrufer
/// bekommt erst weiter, wenn sein Auftrag abgeschlossen ist.
pub fn run_download<R: Runtime>(
    app: AppHandle<R>,
    job_id: u64,
    request: DownloadRequest,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let tools = match resolve_tools(Some(&app)) {
        Ok(v) => v,
        Err(e) => {
            emit_finished(&app, job_id, Err(e.clone()), String::new(), Vec::new());
            return Err(e);
        }
    };
    let settings = request.settings.clone();
    let output_dir = PathBuf::from(&settings.output_dir);
    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        let msg = format!("Der Downloadordner konnte nicht erstellt werden: {e}");
        emit_finished(&app, job_id, Err(msg.clone()), msg.clone(), Vec::new());
        return Err(msg);
    }

    let status_file = tools.runtime_dir.join(format!("job-{job_id}-status.txt"));
    let _ = std::fs::remove_file(&status_file);

    let mut cmd = hidden_command(&tools.yt_dlp);
    cmd.env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUNBUFFERED", "1")
        .arg("--ignore-config")
        .arg("--js-runtimes")
        .arg(format!("deno:{}", tools.deno.display()));

    let is_batch = request.playlist_batch.is_some();
    let mut source_template = settings.filename_template.clone();
    let mut folder_template = settings.video_folder_template.clone();
    if !is_batch {
        if let Some(index) = request.playlist_index {
            source_template = source_template.replace("{playlist_index}", &format!("{index:02}"));
            folder_template = folder_template.replace("{playlist_index}", &format!("{index:02}"));
        }
    }
    let filename_template = template_to_ytdlp(&source_template, settings.filename_max_len as usize);
    let output_template = if settings.separate_folder_per_video && !is_batch {
        let folder = template_component_to_ytdlp(&folder_template, 120);
        format!("{folder}/{filename_template}")
    } else {
        filename_template
    };

    cmd.arg("--newline")
        .arg("--progress")
        .arg("--progress-delta")
        .arg("0.2")
        .arg("--windows-filenames")
        .arg("--trim-filenames")
        .arg(settings.filename_max_len.clamp(40, 240).to_string())
        .arg("--retries")
        .arg(settings.retries.clamp(0, 50).to_string())
        .arg("--fragment-retries")
        .arg(settings.retries.clamp(0, 50).to_string())
        .arg("--retry-sleep")
        .arg("http:exp=1:20")
        .arg("--retry-sleep")
        .arg("fragment:exp=1:10")
        .arg("--concurrent-fragments")
        .arg(settings.concurrent_fragments.clamp(1, 32).to_string())
        .arg("--progress-template")
        .arg(PROGRESS_TEMPLATE)
        .arg("--progress-template")
        .arg("postprocess:POSTPROCESS:%(progress.status)s")
        .arg("--print-to-file")
        .arg("before_dl:TITLE:%(playlist_index)s|%(title)s")
        .arg(&status_file)
        .arg("--print-to-file")
        .arg("after_move:FILE:%(filepath)s")
        .arg(&status_file)
        .arg("--ffmpeg-location")
        .arg(&tools.runtime_dir)
        .arg("--paths")
        .arg(&output_dir)
        .arg("--output")
        .arg(output_template);

    if settings.turbo_downloads {
        let connections = settings.aria2_connections.clamp(2, 16);
        cmd.arg("--throttled-rate")
            .arg("100K")
            .arg("--downloader")
            .arg(format!("http,https:{}", tools.aria2.display()))
            .arg("--downloader")
            .arg("dash,m3u8:native")
            .arg("--downloader-args")
            .arg(ARIA2_ARGS.replace("{n}", &connections.to_string()));
    }
    if let Some(kib) = settings.speed_limit_kib {
        if kib > 0 {
            cmd.arg("--limit-rate").arg(format!("{kib}K"));
        }
    }
    if settings.download_archive {
        cmd.arg("--download-archive")
            .arg(tools.runtime_dir.join("downloaded.txt"));
    }
    if !settings.extractor_args.trim().is_empty() {
        cmd.arg("--extractor-args")
            .arg(settings.extractor_args.trim());
    }

    match settings.mode {
        DownloadMode::Video => {
            let selector = if !is_batch
                && (request.selected_video_format.is_some()
                    || request.selected_audio_format.is_some())
            {
                match (
                    &request.selected_video_format,
                    &request.selected_audio_format,
                ) {
                    (Some(v), Some(a)) => format!("{v}+{a}"),
                    (Some(v), None) => v.clone(),
                    (None, Some(a)) => a.clone(),
                    _ => video_format_selector(settings.video_quality, settings.video_container),
                }
            } else {
                video_format_selector(settings.video_quality, settings.video_container)
            };
            cmd.arg("--format").arg(selector);
            cmd.arg("--merge-output-format")
                .arg(settings.video_container.yt_name());
        }
        DownloadMode::Audio => {
            cmd.arg("--format")
                .arg(
                    request
                        .selected_audio_format
                        .clone()
                        .filter(|_| !is_batch)
                        .unwrap_or_else(|| "bestaudio/best".into()),
                )
                .arg("--extract-audio")
                .arg("--audio-format")
                .arg(settings.audio_format.yt_name())
                .arg("--audio-quality")
                .arg(format!("{}K", settings.audio_quality.clamp(64, 320)));
        }
    }

    if settings.embed_thumbnail {
        cmd.arg("--embed-thumbnail");
    }
    if settings.embed_metadata {
        cmd.arg("--embed-metadata");
    }
    if settings.embed_chapters {
        cmd.arg("--embed-chapters");
    }
    if settings.write_info_json {
        cmd.arg("--write-info-json");
    }
    if settings.write_description {
        cmd.arg("--write-description");
    }
    if settings.download_subtitles {
        cmd.arg("--write-subs");
        if settings.auto_subtitles {
            cmd.arg("--write-auto-subs");
        }
        cmd.arg("--sub-langs")
            .arg(settings.subtitle_languages.trim())
            .arg("--convert-subs")
            .arg(settings.subtitle_format.trim());
        if settings.mode == DownloadMode::Video {
            cmd.arg("--embed-subs");
        }
    }

    // Der Index wandert vor die Daten, damit die Datei sofort überall abspielbar ist.
    if settings.faststart {
        match settings.mode {
            DownloadMode::Video if settings.video_container == VideoContainer::Mp4 => {
                cmd.arg("--postprocessor-args")
                    .arg("Merger:-movflags +faststart");
            }
            DownloadMode::Audio if settings.audio_format == AudioFormat::M4a => {
                cmd.arg("--postprocessor-args")
                    .arg("FFmpegExtractAudio:-movflags +faststart");
            }
            _ => {}
        }
    }

    let source_url = match &request.playlist_batch {
        Some(batch) => {
            cmd.arg("--yes-playlist")
                .arg("--playlist-items")
                .arg(&batch.items_spec);
            batch.playlist_url.clone()
        }
        None => {
            cmd.arg("--no-playlist");
            request.url.clone()
        }
    };

    if !settings.extra_ytdlp_args.trim().is_empty() {
        for arg in split_args(settings.extra_ytdlp_args.trim()) {
            cmd.arg(arg);
        }
    }

    // Die Ausgabe der Engine landet in Dateien statt in Pipes. Piped-Prozesse laufen
    // unter Windows gelegentlich in "Invalid argument" beim Schreiben und koennen den
    // Kindprozess blockieren, wenn der Puffer gefuellt ist.
    let out_path = tools
        .runtime_dir
        .join(format!("job-{job_id}-out.log"));
    let err_path = tools
        .runtime_dir
        .join(format!("job-{job_id}-err.log"));
    let out_file = std::fs::File::create(&out_path);
    let err_file = std::fs::File::create(&err_path);
    let (Ok(out_file), Ok(err_file)) = (out_file, err_file) else {
        let msg = "Die Ausgabedateien der Engine konnten nicht angelegt werden.".to_string();
        emit_finished(&app, job_id, Err(msg.clone()), msg.clone(), Vec::new());
        return Err(msg);
    };

    cmd.arg(&source_url)
        .stdout(Stdio::from(out_file))
        .stderr(Stdio::from(err_file))
        .stdin(Stdio::null());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("Die Download-Engine konnte nicht gestartet werden: {e}");
            emit_finished(&app, job_id, Err(msg.clone()), msg.clone(), Vec::new());
            return Err(msg);
        }
    };
    let pid = child.id();
    // Der Prozess wird gemerkt, damit die App ihn beim Schließen beenden kann.
    if let Some(state) = app.try_state::<crate::AppState>() {
        state.remember_pid(job_id, pid);
    }

    let errors = Arc::new(Mutex::new(String::new()));
    let tracker = Arc::new(Mutex::new(ProgressTracker::default()));
    let batch = request.playlist_batch.clone();
    let batch_total = batch.as_ref().map(|b| b.total).unwrap_or(1);
    let batch_indices = batch
        .as_ref()
        .map(|b| b.indices.clone())
        .unwrap_or_default();
    let mut out_tail = Tail::new(out_path.clone());
    let mut err_tail = Tail::new(err_path.clone());

    let mut status_seen: Vec<String> = Vec::new();
    let mut output_files: Vec<PathBuf> = Vec::new();

    // Liest die beiden Ausgabedateien ein und meldet sie an die Oberfläche.
    let pump_output = |out: &mut Tail, err: &mut Tail, files: &mut Vec<PathBuf>| {
        for line in out.lines() {
            // Die Engine nennt den fertigen Dateinamen im Text. Das ist die
            // verlässlichste Quelle, weil sie unabhängig von der Metadatatei
            // immer vorhanden ist.
            if let Some(path) = extract_destination(&line) {
                if !files.iter().any(|known| known == &path) {
                    files.push(path);
                }
            }
            handle_engine_line(
                &app,
                job_id,
                &line,
                &tracker,
                batch_total,
                &batch_indices,
            );
        }
        for line in err.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(mut raw) = errors.lock() {
                raw.push_str(&line);
                raw.push('\n');
            }
            let level = if line.contains("ERROR:") {
                "error"
            } else if line.contains("WARNING:") {
                "warning"
            } else {
                "info"
            };
            let _ = app.emit(
                "download://log",
                LogPayload {
                    job_id,
                    level: level.into(),
                    line,
                },
            );
        }
    };

    let final_status = loop {
        if cancel.load(Ordering::Relaxed) {
            kill_process_tree(pid, &mut child);
            break None;
        }
        pump_output(&mut out_tail, &mut err_tail, &mut output_files);
        drain_status_file(
            &status_file,
            &mut status_seen,
            &app,
            job_id,
            &mut output_files,
        );
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Err(_) => {
                kill_process_tree(pid, &mut child);
                break None;
            }
        }
    };
    // Rest der Ausgabe noch einlesen, der Prozess ist ja gerade fertig.
    pump_output(&mut out_tail, &mut err_tail, &mut output_files);

    drain_status_file(
        &status_file,
        &mut status_seen,
        &app,
        job_id,
        &mut output_files,
    );
    // Wenn die Engine den Namen nicht gemeldet hat, reicht der Titel, um die
    // fertige Datei zu finden. So bleibt das Tagging zuverlässig.
    if output_files.is_empty() {
        if let Some(path) = expected_output_file(&settings, request.metadata.as_ref()) {
            if path.exists() {
                output_files.push(path);
            }
        }
    }
    let _ = std::fs::remove_file(&status_file);
    if let Some(state) = app.try_state::<crate::AppState>() {
        state.forget_pid(job_id);
    }

    let technical = errors.lock().map(|s| s.clone()).unwrap_or_default();
    let produced = output_files.iter().any(|p| p.exists());
    let mut outcome = if cancel.load(Ordering::Relaxed) {
        Err("Download wurde abgebrochen.".to_string())
    } else if produced || final_status.map(|s| s.success()).unwrap_or(false) {
        Ok(String::new())
    } else {
        Err(friendly_error(&technical))
    };

    // Zusätzliche Metadaten (etwa aus Spotify) nach dem Download einspielen.
    if outcome.is_ok() {
        if let Some(metadata) = request.metadata.as_ref() {
            // Die letzte Datei, die wirklich da ist, ist die fertige Datei.
            let file = output_files
                .iter()
                .rev()
                .find(|path| path.exists())
                .cloned()
                .or_else(|| {
                    expected_output_file(&settings, request.metadata.as_ref())
                        .filter(|path| path.exists())
                });
            if let Some(file) = &file {
                let cover = metadata
                    .artwork_path
                    .as_ref()
                    .map(|path| PathBuf::from(path))
                    .filter(|path| path.is_file());
                let tagging = crate::engine::tags::apply_metadata(&app, file, metadata);
                if let Err(error) = &tagging {
                    // Bei einem Fehler bleibt eine Notiz stehen, damit man
                    // nachsehen kann, was genau nicht geklappt hat.
                    let _ = std::fs::write(
                        tools.runtime_dir.join(format!("job-{job_id}-meta.log")),
                        format!(
                            "datei={}\ncover={:?}\ncover_existiert={}\nfehler={error}\n",
                            file.display(),
                            metadata.artwork_path,
                            cover.is_some(),
                        ),
                    );
                    outcome = Err(error.clone());
                }
            }
        }
    }
    // Bei einem Fehler bleiben die Protokolldateien stehen, damit man nachsehen
    // kann, was die Engine gemeldet hat.
    if outcome.is_ok() {
        let _ = std::fs::remove_file(&out_path);
        let _ = std::fs::remove_file(&err_path);
    }
    let result = match &outcome {
        Ok(_) => Ok(()),
        Err(message) => Err(message.clone()),
    };
    emit_finished(&app, job_id, outcome, technical, output_files);
    result
}

fn emit_finished<R: Runtime>(
    app: &AppHandle<R>,
    job_id: u64,
    outcome: Result<String, String>,
    technical: String,
    files: Vec<PathBuf>,
) {
    let (ok, error, output_file, output_files) = match outcome {
        Ok(_) => (
            true,
            None,
            files.last().map(|p| p.to_string_lossy().into_owned()),
            files
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
        ),
        Err(message) => (false, Some(message), None, Vec::new()),
    };
    let _ = app.emit(
        "download://finished",
        FinishedPayload {
            job_id,
            ok,
            error,
            technical_details: technical,
            output_file,
            output_files,
        },
    );
}

/// Die Sonderzeichen der Windows-Codepage 1252 (0x80–0x9F).
///
/// Die Engine schreibt ihre Ausgabe mit cp1252, nicht mit UTF-8. Für Umlaute wie "ä"
/// entstehen dadurch einzelne Bytes, die kein gültiges UTF-8 sind.
const CP1252_HIGH: [char; 32] = [
    '\u{20AC}', '\u{81}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{8D}', '\u{017D}', '\u{8F}',
    '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{9D}', '\u{017E}', '\u{0178}',
];

/// Wandelt die Ausgabe der Engine in Text um.
///
/// Gültiges UTF-8 wird verwendet. Andernfalls wird cp1252 angenommen — dort landen
/// die Umlaute, weil die Engine ihre Ausgabe nicht als UTF-8 schreibt. Wichtig ist
/// vor allem, dass die Ausgabe niemals abbricht.
fn decode_output(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }
    let mut out = String::with_capacity(bytes.len());
    for byte in bytes {
        match byte {
            0x80..=0x9F => out.push(CP1252_HIGH[(*byte - 0x80) as usize]),
            other => out.push(*other as char),
        }
    }
    out
}

/// Sucht den fertigen Dateinamen in einer Zeile der Engine-Ausgabe.
///
/// Die Engine nennt den Zielpfad an mehreren Stellen: beim Download, beim
/// Zusammenfügen der Formate und beim Einbetten des Bildes. Wer sich auf eine
/// dieser Stellen verlässt, bekommt in jedem Fall eine echte Datei.
fn extract_destination(line: &str) -> Option<PathBuf> {
    let line = line.trim();
    let candidates = [
        ("[ExtractAudio] Destination: ", false),
        ("[download] Destination: ", false),
        ("Merging formats into ", true),
        ("Adding thumbnail to ", true),
        ("Adding metadata to ", true),
    ];
    for (marker, quoted) in candidates {
        let Some(rest) = line.find(marker) else {
            continue;
        };
        let rest = &line[rest + marker.len()..];
        let rest = rest.trim();
        let path = if quoted {
            rest.trim_matches('"').trim()
        } else {
            rest
        };
        if path.is_empty() {
            continue;
        }
        // Ein echter Pfad enthält mindestens einen Trenner.
        if !(path.contains('\\') || path.contains('/')) {
            continue;
        }
        let path = PathBuf::from(path);
        if path.extension().is_some() {
            return Some(path);
        }
    }
    None
}

/// Errät den Namen der fertigen Datei aus den Einstellungen.
///
/// Das greift, wenn die Engine den Pfad nicht meldet. Der Titel wird genauso
/// umgeschrieben wie beim Download, sonst passt der Name nicht.
fn expected_output_file(
    settings: &Settings,
    metadata: Option<&MediaMetadata>,
) -> Option<PathBuf> {
    let metadata = metadata?;
    if metadata.title.trim().is_empty() {
        return None;
    }
    let name = crate::engine::naming::safe_filename(&metadata.title);
    let extension = match settings.mode {
        DownloadMode::Audio => settings.audio_format.yt_name(),
        DownloadMode::Video => settings.video_container.yt_name(),
    };
    Some(PathBuf::from(&settings.output_dir).join(format!("{name}.{extension}")))
}

/// Liest eine Datei schrittweise mit.
///
/// Die Ausgabe des Kindprozesses landet in einer Datei statt in einer Pipe. Das ist
/// robuster: Pipes koennen unter Windows blockieren oder dem Kindprozess beim
/// Schreiben mit "Invalid argument" begegnen, Dateien nicht.
struct Tail {
    path: PathBuf,
    pos: u64,
    pending: Vec<u8>,
}

impl Tail {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            pos: 0,
            pending: Vec::new(),
        }
    }

    /// Liefert die seit dem letzten Aufruf dazugekommenen Zeilen.
    fn lines(&mut self) -> Vec<String> {
        let Ok(mut file) = std::fs::File::open(&self.path) else {
            return Vec::new();
        };
        if file.seek(SeekFrom::Start(self.pos)).is_err() {
            return Vec::new();
        }
        let mut chunk = Vec::new();
        if file.read_to_end(&mut chunk).is_err() {
            return Vec::new();
        }
        self.pos += chunk.len() as u64;
        // Die Bytes bleiben unangetastet, bis eine ganze Zeile da ist. Sonst
        // könnte ein Umlaut mitten im Lesevorgang zerrissen werden.
        self.pending.extend_from_slice(&chunk);
        let mut out = Vec::new();
        while let Some(index) = self.pending.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=index).collect();
            out.push(decode_output(&line).trim_end_matches(['\r', '\n']).to_owned());
        }
        out
    }
}

/// Liest die Metadatendatei, die die Engine während des Laufs schreibt.
///
/// Die Engine schreibt die Datei neu, statt sie fortzuschreiben. Deshalb wird
/// immer die ganze Datei gelesen und nur verarbeitet, was noch nicht bekannt war.
fn drain_status_file<R: Runtime>(
    path: &Path,
    seen: &mut Vec<String>,
    app: &AppHandle<R>,
    job_id: u64,
    output_files: &mut Vec<PathBuf>,
) {
    let Ok(mut file) = std::fs::File::open(path) else {
        return;
    };
    let mut content = Vec::new();
    if file.read_to_end(&mut content).is_err() {
        return;
    }
    let text = decode_output(&content);
    let lines: Vec<String> = text.lines().map(|line| line.trim().to_owned()).collect();
    // Wurde die Datei neu geschrieben, fängt die Liste wieder von vorn an.
    let known = seen.len() <= lines.len() && lines[..seen.len()] == seen[..];
    let fresh = if known {
        &lines[seen.len()..]
    } else {
        &lines[..]
    };
    seen.clear();
    seen.extend(lines.iter().cloned());
    for line in fresh {
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("TITLE:") {
            let (index, title) = match rest.split_once('|') {
                Some((i, t)) => (parse_number::<u32>(i), t.trim()),
                None => (None, rest.trim()),
            };
            if !title.is_empty() {
                let _ = app.emit(
                    "download://title",
                    TitlePayload {
                        job_id,
                        title: title.to_owned(),
                        playlist_index: index,
                    },
                );
            }
        } else if let Some(rest) = line.strip_prefix("FILE:") {
            let path = PathBuf::from(rest.trim());
            if !output_files.iter().any(|p| p == &path) {
                output_files.push(path.clone());
            }
            let _ = app.emit(
                "download://file",
                FilePayload {
                    job_id,
                    path: path.to_string_lossy().into_owned(),
                },
            );
        }
    }
}

fn handle_engine_line<R: Runtime>(
    app: &AppHandle<R>,
    job_id: u64,
    line: &str,
    tracker: &Arc<Mutex<ProgressTracker>>,
    batch_total: u32,
    batch_indices: &[u32],
) {
    if let Some(rest) = line.strip_prefix("PROGRESS:") {
        let mut parts = rest.split('|');
        let status = parts.next().unwrap_or("").trim();
        let downloaded = parse_number::<u64>(parts.next().unwrap_or(""));
        let total = parse_number::<u64>(parts.next().unwrap_or(""));
        let estimate = parse_number::<u64>(parts.next().unwrap_or(""));
        let speed = parse_number::<f64>(parts.next().unwrap_or(""));
        let eta = parse_number::<u64>(parts.next().unwrap_or(""));
        let playlist_index = parse_number::<u32>(parts.next().unwrap_or(""));

        let (value, total_bytes) = {
            let mut guard = tracker.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(index) = playlist_index {
                if guard.playlist_index.is_some() && guard.playlist_index != Some(index) {
                    guard.reset_streams();
                }
            }
            guard.push(downloaded, total.or(estimate));
            guard.playlist_index = playlist_index.or(guard.playlist_index);
            let file_fraction = guard.fraction().unwrap_or(0.0);
            let overall = overall_value(&guard, batch_total, batch_indices, file_fraction);
            let total_bytes = guard.finished_total + guard.current_total;
            (guard.report(overall), total_bytes)
        };

        let _ = app.emit(
            "download://progress",
            ProgressPayload {
                job_id,
                value: value.min(0.999),
                state: JobState::Downloading,
                status: match status {
                    "finished" => "Stream abgeschlossen".to_string(),
                    "downloading" => "Daten werden heruntergeladen".to_string(),
                    other => format!("Status: {other}"),
                },
                speed: speed
                    .filter(|s| *s > 0.0)
                    .map(format_speed)
                    .unwrap_or_default(),
                eta: eta.filter(|s| *s > 0).map(format_eta).unwrap_or_default(),
                total: if total_bytes > 0 {
                    format_bytes(total_bytes)
                } else {
                    String::new()
                },
            },
        );
        return;
    }

    if let Some(rest) = line.strip_prefix("POSTPROCESS:") {
        let value = {
            let mut guard = tracker.lock().unwrap_or_else(|e| e.into_inner());
            let overall = overall_value(&guard, batch_total, batch_indices, 1.0);
            guard.report(overall)
        };
        let _ = app.emit(
            "download://progress",
            ProgressPayload {
                job_id,
                value,
                state: JobState::PostProcessing,
                status: format!("Verarbeitung: {}", rest.trim()),
                speed: String::new(),
                eta: String::new(),
                total: String::new(),
            },
        );
        return;
    }

    if line.trim().is_empty() {
        return;
    }

    if let Some((state, status)) = status_from_engine_line(line) {
        let value = {
            let mut guard = tracker.lock().unwrap_or_else(|e| e.into_inner());
            let file_fraction = guard.fraction().unwrap_or(0.0);
            let overall = overall_value(&guard, batch_total, batch_indices, file_fraction);
            guard.report(overall)
        };
        let _ = app.emit(
            "download://progress",
            ProgressPayload {
                job_id,
                value,
                state,
                status,
                speed: String::new(),
                eta: String::new(),
                total: String::new(),
            },
        );
    }

    let level = if line.contains("ERROR:") {
        "error"
    } else if line.contains("WARNING:") {
        "warning"
    } else {
        "debug"
    };
    let _ = app.emit(
        "download://log",
        LogPayload {
            job_id,
            level: level.into(),
            line: line.to_owned(),
        },
    );
}

fn parse_number<T: std::str::FromStr>(text: &str) -> Option<T> {
    let text = text.trim();
    if text.is_empty() || text == "NA" || text == "None" || text == "unknown" {
        return None;
    }
    text.parse::<T>().ok()
}

fn status_from_engine_line(line: &str) -> Option<(JobState, String)> {
    if line.contains("[Merger]") {
        Some((
            JobState::PostProcessing,
            "Video und Audio werden zusammengeführt".into(),
        ))
    } else if line.contains("[ExtractAudio]") {
        Some((JobState::PostProcessing, "Audio wird konvertiert".into()))
    } else if line.contains("[Metadata]") {
        Some((
            JobState::PostProcessing,
            "Metadaten werden eingebettet".into(),
        ))
    } else if line.contains("[EmbedThumbnail]") {
        Some((JobState::PostProcessing, "Cover wird eingebettet".into()))
    } else if line.contains("[EmbedSubtitle]") {
        Some((
            JobState::PostProcessing,
            "Untertitel werden eingebettet".into(),
        ))
    } else if line.contains("[download] Destination:") {
        Some((
            JobState::Downloading,
            "Media-Stream wird heruntergeladen".into(),
        ))
    } else {
        None
    }
}

/// Zerlegt eine Argumentliste in einzelne Argumente.
fn split_args(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in input.chars() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => current.push(ch),
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn video_format_selector(quality: VideoQuality, container: VideoContainer) -> String {
    if quality == VideoQuality::Smallest {
        return match container {
            VideoContainer::Mp4 => "worst[ext=mp4]/worst".into(),
            VideoContainer::Webm => "worst[ext=webm]/worst".into(),
            VideoContainer::Mkv => "worst".into(),
        };
    }
    let limit = quality
        .max_height()
        .map(|h| format!("[height<={h}]"))
        .unwrap_or_default();
    match container {
        VideoContainer::Mp4 => {
            format!("bv*{limit}[ext=mp4]+ba[ext=m4a]/b{limit}[ext=mp4]/bv*{limit}+ba/b{limit}/b")
        }
        VideoContainer::Webm => {
            format!("bv*{limit}[ext=webm]+ba[ext=webm]/b{limit}[ext=webm]/bv*{limit}+ba/b{limit}/b")
        }
        VideoContainer::Mkv => format!("bv*{limit}+ba/b{limit}/b"),
    }
}

/// Erzeugt ein Dateinamen-Template für die Engine.
pub fn template_component_to_ytdlp(template: &str, max_len: usize) -> String {
    sanitize_filename(template, max_len.max(40))
        .replace("{title}", "%(title)s")
        .replace("{channel}", "%(channel,uploader)s")
        .replace("{date}", "%(upload_date)s")
        .replace("{id}", "%(id)s")
        .replace("{playlist_index}", "%(playlist_index)02d")
}

/// Erzeugt ein vollständiges Dateinamen-Template für die Engine.
pub fn template_to_ytdlp(template: &str, max_len: usize) -> String {
    let mut t = template_component_to_ytdlp(template, max_len);
    if !t.contains("%(ext)s") {
        t.push_str(".%(ext)s");
    }
    if max_len > 0 {
        t
    } else {
        "%(title)s [%(id)s].%(ext)s".into()
    }
}

/// Entfernt unzulässige Zeichen aus Dateinamen.
pub fn sanitize_filename(input: &str, max_len: usize) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    let trimmed = out.trim().trim_end_matches(['.', ' ']).to_owned();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let mut result = if reserved.iter().any(|r| trimmed.eq_ignore_ascii_case(r)) {
        format!("_{trimmed}")
    } else {
        trimmed
    };
    if result.is_empty() {
        result = "download".into();
    }
    if result.chars().count() > max_len {
        result = result.chars().take(max_len).collect();
    }
    result
}

/// Formatiert eine Übertragungsrate.
pub fn format_speed(bytes_per_second: f64) -> String {
    if !bytes_per_second.is_finite() || bytes_per_second <= 0.0 {
        return String::new();
    }
    format!("{}/s", format_bytes(bytes_per_second as u64))
}

/// Formatiert eine Restzeit.
pub fn format_eta(seconds: u64) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Formatiert eine Dateigröße.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut n = bytes as f64;
    let mut i = 0;
    while n >= 1024.0 && i < UNITS.len() - 1 {
        n /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", bytes, UNITS[i])
    } else {
        format!("{n:.1} {}", UNITS[i])
    }
}

/// Beendet den Prozess samt Kindprozessen.
fn kill_process_tree(pid: u32, child: &mut std::process::Child) {
    #[cfg(target_os = "windows")]
    {
        let pid_s = pid.to_string();
        let _ = hidden_command("taskkill")
            .args(["/PID", pid_s.as_str(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = child.wait();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Nur damit die Warnung zu EngineTools nicht erscheint.
#[allow(dead_code)]
fn _unused(_: &EngineTools) {}
