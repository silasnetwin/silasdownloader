//! Lokaler Media-Konverter auf Basis von FFmpeg.

use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Stdio,
    thread,
    time::Duration,
};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::{
    engine::tools::{hidden_command, resolve_tools},
    model::ConvertFormat,
};

/// Meldet den Fortschritt einer Konvertierung.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertProgress {
    pub value: f32,
    pub status: String,
}

/// Konvertiert eine Datei und liefert den Pfad der fertigen Datei.
pub fn convert(
    app: &AppHandle,
    input: &Path,
    output_dir: &Path,
    format: ConvertFormat,
) -> Result<PathBuf, String> {
    let tools = resolve_tools(Some(app))?;
    std::fs::create_dir_all(output_dir).map_err(|e| format!("Ausgabeordner: {e}"))?;
    let duration = probe_duration(&tools.ffprobe, input).unwrap_or(0.0);
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("converted");
    let output = unique_output_path(output_dir, stem, format.extension());

    let mut cmd = hidden_command(&tools.ffmpeg);
    cmd.arg("-nostdin").arg("-y").arg("-i").arg(input);
    if is_audio_only(format) {
        cmd.arg("-vn");
        match format {
            ConvertFormat::Mp3 => {
                cmd.args(["-c:a", "libmp3lame", "-b:a", "256k"]);
            }
            ConvertFormat::M4a => {
                cmd.args(["-c:a", "aac", "-b:a", "256k"]);
            }
            ConvertFormat::Wav => {
                cmd.args(["-c:a", "pcm_s16le"]);
            }
            ConvertFormat::Flac => {
                cmd.args(["-c:a", "flac"]);
            }
            _ => {}
        }
    } else {
        match format {
            ConvertFormat::Mp4 | ConvertFormat::Mov => {
                cmd.args([
                    "-c:v",
                    "libx264",
                    "-preset",
                    "medium",
                    "-crf",
                    "20",
                    "-c:a",
                    "aac",
                    "-b:a",
                    "192k",
                    "-movflags",
                    "+faststart",
                ]);
            }
            ConvertFormat::Webm => {
                cmd.args([
                    "-c:v",
                    "libvpx-vp9",
                    "-crf",
                    "30",
                    "-b:v",
                    "0",
                    "-c:a",
                    "libopus",
                ]);
            }
            ConvertFormat::Mkv => {
                cmd.args(["-c", "copy"]);
            }
            _ => {}
        }
    }
    cmd.args(["-progress", "pipe:1", "-nostats"])
        .arg(&output)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("FFmpeg konnte nicht gestartet werden: {e}"))?;
    let stdout = child.stdout.take();
    let (tx, rx) = std::sync::mpsc::channel::<f64>();
    thread::spawn(move || {
        if let Some(s) = stdout {
            for line in BufReader::new(s).lines().map_while(Result::ok) {
                if let Some(value) = line
                    .strip_prefix("out_time_ms=")
                    .and_then(|x| x.parse::<f64>().ok())
                {
                    let _ = tx.send(value / 1_000_000.0);
                }
            }
        }
    });
    loop {
        while let Ok(seconds) = rx.try_recv() {
            let value = if duration > 0.0 {
                (seconds / duration) as f32
            } else {
                0.0
            };
            let _ = app.emit(
                "convert://progress",
                ConvertProgress {
                    value: value.clamp(0.0, 0.99),
                    status: format!("Konvertiert · {:.0} %", value * 100.0),
                },
            );
        }
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                let _ = app.emit(
                    "convert://progress",
                    ConvertProgress {
                        value: 1.0,
                        status: "Fertig".into(),
                    },
                );
                return Ok(output);
            }
            Ok(Some(_)) => {
                let _ = std::fs::remove_file(&output);
                return Err("FFmpeg konnte die Datei nicht konvertieren.".into());
            }
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Err(e) => return Err(format!("FFmpeg-Prozessfehler: {e}")),
        }
    }
}

/// Prüft, ob das Zielformat nur Audio enthält.
pub fn is_audio_only(format: ConvertFormat) -> bool {
    matches!(
        format,
        ConvertFormat::Mp3 | ConvertFormat::M4a | ConvertFormat::Wav | ConvertFormat::Flac
    )
}

/// Findet einen freien Dateinamen im Ausgabeordner.
fn unique_output_path(dir: &Path, stem: &str, extension: &str) -> PathBuf {
    let base = sanitize(stem, 180);
    let first = dir.join(format!("{base}.{extension}"));
    if !first.exists() {
        return first;
    }
    for index in 2..=9999 {
        let candidate = dir.join(format!("{base} ({index}).{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!(
        "{base}-{}.{}",
        chrono::Utc::now().timestamp(),
        extension
    ))
}

/// Entfernt unzulässige Zeichen aus Dateinamen.
fn sanitize(input: &str, max_len: usize) -> String {
    let mut out: String = input
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }
    if out.chars().count() > max_len {
        out = out.chars().take(max_len).collect();
    }
    out
}

/// Liest die Laufzeit einer Datei.
fn probe_duration(ffprobe: &Path, input: &Path) -> Option<f64> {
    let out = hidden_command(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}
