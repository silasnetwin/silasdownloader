//! Ort der mitgelieferten Werkzeuge (yt-dlp, FFmpeg, Deno, aria2c).
//!
//! Die Binaries liegen als Tauri-Ressource neben der EXE und werden zur Laufzeit
//! aufgelöst, damit der Build schnell bleibt und die App trotzdem portabel ist.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Clone)]
pub struct EngineTools {
    pub yt_dlp: PathBuf,
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub deno: PathBuf,
    pub aria2: PathBuf,
    pub runtime_dir: PathBuf,
}

/// Mögliche Orte, an denen die Werkzeuge liegen können.
fn candidates<R: Runtime>(app: Option<&AppHandle<R>>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(app) = app {
        if let Ok(dir) = app.path().resource_dir() {
            out.push(dir.join("bin"));
            out.push(dir.join("resources").join("bin"));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            out.push(parent.join("resources").join("bin"));
            out.push(parent.join("bin"));
        }
    }
    out.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("bin"),
    );
    out
}

/// Löst die Werkzeuge auf und liefert ihre Pfade.
///
/// Ohne laufende App wird der Ordner neben der EXE bzw. im Projekt geprüft.
pub fn resolve_tools<R: Runtime>(app: Option<&AppHandle<R>>) -> Result<EngineTools, String> {
    let mut found: Option<PathBuf> = None;
    for dir in candidates(app) {
        if dir.join("yt-dlp.exe").is_file() {
            found = Some(dir);
            break;
        }
    }
    let dir = found.ok_or(
        "Die Download-Werkzeuge wurden nicht gefunden. Bitte die App neu bauen \
         (scripts/fetch-tools.ps1) oder die EXE zusammen mit dem Ordner 'resources' verteilen."
            .to_string(),
    )?;
    let runtime_dir = dir.clone();
    clean_stale_logs(&runtime_dir);
    Ok(EngineTools {
        yt_dlp: dir.join("yt-dlp.exe"),
        ffmpeg: dir.join("ffmpeg.exe"),
        ffprobe: dir.join("ffprobe.exe"),
        deno: dir.join("deno.exe"),
        aria2: dir.join("aria2c.exe"),
        runtime_dir,
    })
}

/// Entfernt Reste abgebrochener Läufe.
///
/// Die Ausgabe der Engine liegt in Dateien. Bleiben sie nach einem Absturz liegen,
/// füllen sie den Cache.
fn clean_stale_logs(runtime_dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(runtime_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with("job-")
            && (name.ends_with("-out.log")
                || name.ends_with("-err.log")
                || name.ends_with("-status.txt"))
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Startet einen Prozess ohne sichtbares Fenster.
pub fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Liest die Version von yt-dlp.
pub fn ytdlp_version(yt_dlp: &Path) -> String {
    hidden_command(yt_dlp)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|| "unbekannt".to_string())
}

/// Aktualisiert yt-dlp auf die neueste Version.
///
/// yt-dlp geht alle paar Wochen kaputt, wenn sich eine Seite ändert. Die neueste
/// Version kommt von den GitHub-Releases. Vor dem Tausch wird geprüft, ob die
/// neue Datei überhaupt läuft.
pub fn update_ytdlp(yt_dlp: &Path) -> Result<String, String> {
    let url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
    let nebenbei = yt_dlp.with_extension("neu.exe");
    let _ = std::fs::remove_file(&nebenbei);
    let output = hidden_command("curl.exe")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg("180")
        .arg("--output")
        .arg(&nebenbei)
        .arg("--")
        .arg(url)
        .output()
        .map_err(|e| format!("Die neue Version konnte nicht geladen werden: {e}"))?;
    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr);
        let last = details
            .lines()
            .filter(|line| !line.trim().is_empty())
            .last()
            .unwrap_or("unbekannter Fehler")
            .to_owned();
        return Err(format!("Die neue Version konnte nicht geladen werden: {last}"));
    }
    if !nebenbei.is_file() {
        return Err("Die neue Version ist nicht angekommen.".to_string());
    }
    // Die neue Datei muss laufen, sonst wird nicht getauscht.
    let probe = hidden_command(&nebenbei)
        .arg("--version")
        .output()
        .map_err(|e| format!("Die neue Version startet nicht: {e}"))?;
    if !probe.status.success() {
        let _ = std::fs::remove_file(&nebenbei);
        return Err("Die neue Version startet nicht — es wurde nichts geändert.".to_string());
    }
    let version = String::from_utf8_lossy(&probe.stdout).trim().to_owned();
    // Tauschen. Läuft gerade ein Download, ist die Datei gesperrt.
    if let Err(error) = std::fs::copy(&nebenbei, yt_dlp) {
        let _ = std::fs::remove_file(&nebenbei);
        if error.kind() == std::io::ErrorKind::PermissionDenied {
            return Err(
                "yt-dlp ist gerade belegt. Bitte laufende Downloads beenden und erneut versuchen."
                    .to_string(),
            );
        }
        return Err(format!("yt-dlp konnte nicht ersetzt werden: {error}"));
    }
    let _ = std::fs::remove_file(&nebenbei);
    Ok(version)
}
