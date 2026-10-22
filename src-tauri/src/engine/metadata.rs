//! Analyse von Videos und Playlists.

use std::{
    hash::{Hash, Hasher},
    path::PathBuf,
};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
use tauri::{AppHandle, Runtime};

use crate::{
    engine::tools::{hidden_command, resolve_tools},
    model::{collect_language_codes, MediaFormat, PlaylistEntry, PlaylistPreview, VideoPreview},
};

/// Analysiert ein Video und liefert Metadaten, Formate und Vorschaubild.
///
/// Ohne laufende App (`app` ist `None`) wird der Werkzeugordner im Projekt genutzt.
pub fn analyze_video<R: Runtime>(
    app: Option<&AppHandle<R>>,
    url: &str,
    extractor_args: &str,
) -> Result<VideoPreview, String> {
    let tools = resolve_tools(app)?;
    let mut command = hidden_command(&tools.yt_dlp);
    command
        .env("PYTHONIOENCODING", "utf-8")
        .arg("--ignore-config")
        .arg("--js-runtimes")
        .arg(format!("deno:{}", tools.deno.display()))
        .arg("--dump-single-json")
        .arg("--skip-download")
        .arg("--no-warnings")
        .arg("--no-playlist");
    if !extractor_args.trim().is_empty() {
        command.arg("--extractor-args").arg(extractor_args.trim());
    }
    let out = command
        .arg("--ffmpeg-location")
        .arg(&tools.runtime_dir)
        .arg(url)
        .output()
        .map_err(|e| format!("Analyse konnte nicht gestartet werden: {e}"))?;
    if !out.status.success() {
        return Err(friendly_error(&String::from_utf8_lossy(&out.stderr)));
    }
    let value: Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("Metadaten konnten nicht gelesen werden: {e}"))?;

    let mut formats = Vec::new();
    if let Some(items) = value.get("formats").and_then(Value::as_array) {
        for f in items {
            let vcodec = text(f, "vcodec");
            let acodec = text(f, "acodec");
            let height = number(f, "height").map(|x| x as u32);
            let resolution = text(f, "resolution");
            formats.push(MediaFormat {
                id: text(f, "format_id"),
                ext: text(f, "ext"),
                resolution: if resolution.is_empty() {
                    height.map(|h| format!("{h}p")).unwrap_or_default()
                } else {
                    resolution
                },
                height,
                fps: f.get("fps").and_then(Value::as_f64),
                vcodec: vcodec.clone(),
                acodec: acodec.clone(),
                filesize: number(f, "filesize").or_else(|| number(f, "filesize_approx")),
                audio_only: vcodec == "none" && acodec != "none",
                video_only: vcodec != "none" && acodec == "none",
            });
        }
    }
    formats.sort_by(|a, b| {
        b.height.cmp(&a.height).then_with(|| {
            b.fps
                .partial_cmp(&a.fps)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    let thumb = fetch_thumbnail(url, &value, &tools).ok();

    Ok(VideoPreview {
        url: url.into(),
        id: text(&value, "id"),
        title: text(&value, "title"),
        channel: {
            let x = text(&value, "channel");
            if x.is_empty() {
                text(&value, "uploader")
            } else {
                x
            }
        },
        duration: value
            .get("duration")
            .and_then(Value::as_f64)
            .map(|x| x.max(0.0) as u64),
        upload_date: text(&value, "upload_date"),
        thumbnail_data_url: thumb,
        formats,
        subtitle_languages: collect_language_codes(&value),
    })
}

/// Analysiert eine Playlist.
pub fn analyze_playlist<R: Runtime>(
    app: Option<&AppHandle<R>>,
    url: &str,
    extractor_args: &str,
) -> Result<PlaylistPreview, String> {
    let tools = resolve_tools(app)?;
    let mut command = hidden_command(&tools.yt_dlp);
    command
        .env("PYTHONIOENCODING", "utf-8")
        .arg("--ignore-config")
        .arg("--js-runtimes")
        .arg(format!("deno:{}", tools.deno.display()))
        .arg("--flat-playlist")
        .arg("--dump-single-json")
        .arg("--skip-download")
        .arg("--no-warnings");
    if !extractor_args.trim().is_empty() {
        command.arg("--extractor-args").arg(extractor_args.trim());
    }
    let out = command
        .arg(url)
        .output()
        .map_err(|e| format!("Playlist-Analyse konnte nicht gestartet werden: {e}"))?;
    if !out.status.success() {
        return Err(friendly_error(&String::from_utf8_lossy(&out.stderr)));
    }
    let value: Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    if let Some(arr) = value.get("entries").and_then(Value::as_array) {
        for (i, e) in arr.iter().enumerate() {
            let id = text(e, "id");
            let raw = text(e, "url");
            let entry_url = if raw.starts_with("http") {
                raw
            } else if !id.is_empty() {
                format!("https://www.youtube.com/watch?v={id}")
            } else {
                continue;
            };
            entries.push(PlaylistEntry {
                index: (i + 1) as u32,
                id,
                url: entry_url,
                title: text(e, "title"),
                duration: e
                    .get("duration")
                    .and_then(Value::as_f64)
                    .map(|x| x.max(0.0) as u64),
                selected: true,
            });
        }
    }
    Ok(PlaylistPreview {
        url: url.into(),
        title: text(&value, "title"),
        entries,
    })
}

/// Versionen der eingebetteten Engine.
pub fn engine_versions<R: Runtime>(app: &AppHandle<R>) -> (String, String) {
    let Ok(tools) = resolve_tools(Some(app)) else {
        return ("nicht bereitgestellt".into(), "nicht bereitgestellt".into());
    };
    let yd = hidden_command(&tools.yt_dlp)
        .env("PYTHONIOENCODING", "utf-8")
        .arg("--ignore-config")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "unbekannt".into());
    let ff = hidden_command(&tools.ffmpeg)
        .arg("-version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(str::to_owned))
        .map(|line| shorten_ffmpeg_version(&line))
        .unwrap_or_else(|| "unbekannt".into());
    (yd, ff)
}

/// Kürzt die FFmpeg-Versionszeile auf das Wesentliche.
///
/// FFmpeg meldet eine ganze Zeile mit Copyright und Build-Daten. Für die Anzeige
/// reichen Name und Nummer.
fn shorten_ffmpeg_version(line: &str) -> String {
    // "ffmpeg version 9.0-essentials_build-www.gyan.dev" -> "9.0"
    let mut teile = line.split_whitespace();
    let _name = teile.next();
    if teile.next() != Some("version") {
        return line.to_owned();
    }
    match teile.next() {
        Some(version) => {
            // "9.0-essentials_build-www.gyan.dev" -> "9.0"
            let kurz = version
                .split('-')
                .next()
                .unwrap_or(version)
                .to_owned();
            format!("FFmpeg {kurz}")
        }
        None => line.to_owned(),
    }
}

/// Lädt das Vorschaubild direkt vom Bild-CDN.
///
/// Ein zweiter Start der Engine würde die komplette Extraktion wiederholen, deshalb
/// wird das Bild hier direkt geholt und als Data-URL übergeben.
fn fetch_thumbnail(
    url: &str,
    info: &Value,
    tools: &crate::engine::tools::EngineTools,
) -> Result<String, String> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    let dir = tools.runtime_dir.join("previews");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let base = format!("preview-{:x}", hasher.finish());

    if let Some(remote) = best_thumbnail_url(info) {
        let extension = remote
            .rsplit('.')
            .next()
            .filter(|e| e.len() <= 4 && !e.contains('?'))
            .unwrap_or("img")
            .to_ascii_lowercase();
        let target = dir.join(format!("{base}.{extension}"));
        if target.exists() {
            return to_data_url(&target);
        }
        if download_direct(&remote, &target).is_ok() {
            return to_data_url(&target);
        }
    }
    download_with_engine(url, tools, &base)
}

/// Wählt das beste verfügbare Vorschaubild.
fn best_thumbnail_url(info: &Value) -> Option<String> {
    if let Some(list) = info.get("thumbnails").and_then(Value::as_array) {
        if let Some(best) = list
            .iter()
            .filter_map(|t| t.get("url").and_then(Value::as_str))
            .filter(|url| !url.is_empty())
            .last()
        {
            return Some(best.to_owned());
        }
    }
    info.get("thumbnail")
        .and_then(Value::as_str)
        .filter(|url| !url.is_empty())
        .map(str::to_owned)
}

/// Holt eine Datei mit dem HTTP-Client von Windows.
fn download_direct(url: &str, target: &std::path::Path) -> Result<(), String> {
    let output = hidden_command("curl.exe")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg("25")
        .arg("--output")
        .arg(target)
        .arg("--")
        .arg(url)
        .output()
        .map_err(|e| e.to_string())?;
    let size = std::fs::metadata(target).map(|m| m.len()).unwrap_or(0);
    if output.status.success() && size > 1024 {
        Ok(())
    } else {
        let _ = std::fs::remove_file(target);
        Err("Bild konnte nicht geladen werden".into())
    }
}

/// Rückfall: die Engine schreibt das Cover selbst.
fn download_with_engine(
    url: &str,
    tools: &crate::engine::tools::EngineTools,
    base: &str,
) -> Result<String, String> {
    let dir = tools.runtime_dir.join("previews");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let out = hidden_command(&tools.yt_dlp)
        .env("PYTHONIOENCODING", "utf-8")
        .arg("--ignore-config")
        .arg("--js-runtimes")
        .arg(format!("deno:{}", tools.deno.display()))
        .arg("--skip-download")
        .arg("--no-playlist")
        .arg("--write-thumbnail")
        .arg("--convert-thumbnails")
        .arg("png")
        .arg("--paths")
        .arg(&dir)
        .arg("--output")
        .arg(format!("{base}.%(ext)s"))
        .arg("--ffmpeg-location")
        .arg(&tools.runtime_dir)
        .arg(url)
        .output()
        .map_err(|e| e.to_string())?;
    let png = dir.join(format!("{base}.png"));
    if out.status.success() && png.exists() {
        return to_data_url(&png);
    }
    Err("Vorschaubild nicht verfügbar".into())
}

/// Wandelt eine Bilddatei in eine Data-URL.
fn to_data_url(path: &PathBuf) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mime = match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "png" => "image/png",
        "webp" => "image/webp",
        "jpg" | "jpeg" => "image/jpeg",
        _ => "image/png",
    };
    Ok(format!("data:{mime};base64,{}", STANDARD.encode(&bytes)))
}

/// Wandelt eine Playlist in das einheitliche Listenformat um.
pub fn to_list(list: PlaylistPreview) -> crate::model::ListPreview {
    crate::model::ListPreview {
        url: list.url,
        title: list.title,
        list_kind: "playlist".into(),
        seasons: Vec::new(),
        entries: list
            .entries
            .into_iter()
            .enumerate()
            .map(|(position, entry)| crate::model::ListEntry {
                index: entry.index.max((position + 1) as u32),
                title: entry.title,
                url: entry.url,
                duration: entry.duration,
                season: None,
                episode: None,
                selected: true,
            })
            .collect(),
    }
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn number(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

/// Übersetzt technische Fehlermeldungen in verständliche Texte.
pub fn friendly_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("unsupported url") {
        "Die URL wird nicht unterstützt.".into()
    } else if lower.contains("private video") || lower.contains("sign in") {
        "Auf dieses Video kann ohne zusätzliche Berechtigung nicht zugegriffen werden.".into()
    } else if lower.contains("video unavailable") || lower.contains("not available") {
        "Das Video ist nicht verfügbar.".into()
    } else if lower.contains("http error 403") {
        "Der Server hat den Zugriff abgelehnt (HTTP 403). Bitte später erneut versuchen.".into()
    } else if lower.contains("http error 429") {
        "YouTube hat die Anfrage gedrosselt. Weniger parallele Downloads verwenden.".into()
    } else if lower.contains("no space left") {
        "Auf dem Laufwerk ist nicht genügend Speicherplatz verfügbar.".into()
    } else if lower.contains("unable to download") || lower.contains("connection reset") {
        "Die Verbindung wurde während des Downloads unterbrochen.".into()
    } else {
        let line = raw
            .lines()
            .rev()
            .find(|x| !x.trim().is_empty())
            .unwrap_or("");
        let cleaned = strip_ansi(line.trim().trim_start_matches("ERROR:").trim());
        if cleaned.is_empty() {
            "Der Download ist fehlgeschlagen. Details stehen im Protokoll.".into()
        } else {
            cleaned
        }
    }
}

/// Entfernt die ANSI-Farbcodes, mit denen die Engine Fehlermeldungen auszeichnet.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            // ESC [ ... abschliessender Buchstabe
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        out.push(character);
    }
    out.trim().to_owned()
}
