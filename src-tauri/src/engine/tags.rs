//! Schreibt zusätzliche Metadaten in eine fertige Mediendatei.
//!
//! Für Spotify-Tracks stammen Album, Album-Interpret, Tracknummer und Cover aus der
//! Spotify-Seite und ergänzenden Albumdaten. Die Engine bettet die Metadaten des
//! Videos ein, deshalb werden die Spotify-Angaben danach mit FFmpeg überschrieben.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Runtime};

use crate::{
    engine::tools::{hidden_command, resolve_tools},
    model::MediaMetadata,
};

/// Schreibt die Metadaten in die Datei.
///
/// Die Datei wird nicht verändert, wenn kein zusätzlicher Inhalt vorhanden ist.
pub fn apply_metadata<R: Runtime>(
    app: &AppHandle<R>,
    media_path: &Path,
    metadata: &MediaMetadata,
) -> Result<(), String> {
    let tools = resolve_tools(Some(app))?;
    write_tags(&tools.ffmpeg, media_path, metadata)
}

/// Schreibt Titel, Tags und Cover in eine fertige Mediendatei.
///
/// Das FFmpeg-Programm wird übergeben, damit sich die Funktion auch ohne laufende
/// App prüfen lässt.
pub fn write_tags(
    ffmpeg: &Path,
    media_path: &Path,
    metadata: &MediaMetadata,
) -> Result<(), String> {
    let has_tags = !metadata.artist.trim().is_empty()
        || !metadata.album.trim().is_empty()
        || !metadata.album_artist.trim().is_empty()
        || metadata.track_number.is_some()
        || metadata.year.is_some();
    let cover = metadata
        .artwork_path
        .as_ref()
        .map(PathBuf::from)
        .filter(|path| path.is_file());
    let lyrics = metadata
        .lyrics
        .as_deref()
        .map(|text| text.trim())
        .filter(|text| !text.is_empty());
    if !has_tags && cover.is_none() {
        if let Some(text) = lyrics {
            write_lyrics(media_path, text)?;
        }
        return Ok(());
    }

    // Die Endung muss erhalten bleiben. Eine MP3 als "tagged.mp4" zu schreiben,
    // erzeugt einen MP4-Container und die Tags landen nicht mehr in den ID3-Frames.
    let extension = media_path
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("mp4")
        .to_owned();
    let stem = media_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("tagged");
    let tagged_path = media_path.with_file_name(format!("{stem}.tagged.{extension}"));
    let _ = std::fs::remove_file(&tagged_path);

    let mut command = hidden_command(ffmpeg);
    command.arg("-nostdin").arg("-y").arg("-i").arg(media_path);
    let with_cover = match &cover {
        Some(path) => {
            command.arg("-i").arg(path);
            true
        }
        None => false,
    };
    if with_cover {
        command
            .arg("-map")
            .arg("0:a:0")
            .arg("-map")
            .arg("1:v:0")
            .arg("-c:a")
            .arg("copy")
            .arg("-c:v")
            .arg("mjpeg")
            .arg("-q:v")
            .arg("2")
            // Quadratisch zuschneiden, wie ein normales Albumcover.
            .arg("-vf")
            .arg("scale=300:300:force_original_aspect_ratio=increase,crop=300:300")
            .arg("-metadata:s:v")
            .arg("title=Album cover")
            .arg("-metadata:s:v")
            .arg("comment=Cover (front)")
            .arg("-metadata:s:v")
            .arg("mimetype=image/jpeg")
            .arg("-disposition:v:0")
            .arg("attached_pic");
    } else {
        command.arg("-map").arg("0:a").arg("-c:a").arg("copy");
    }
    // Die Engine hat bereits die Tags des YouTube-Videos eingebettet. Ohne diesen
    // Schnitt blieben dessen Album, Jahr, Genre und Beschreibung stehen, wenn
    // Spotify eine Angabe nicht kennt (etwa bei einem einzelnen Track ohne Album).
    command
        .arg("-map_metadata")
        .arg("-1")
        .arg("-id3v2_version")
        .arg("3")
        .arg("-write_id3v1")
        .arg("0");
    for (key, value) in [
        ("title", metadata.title.as_str()),
        ("artist", metadata.artist.as_str()),
        ("album_artist", metadata.album_artist.as_str()),
        ("album", metadata.album.as_str()),
    ] {
        if !value.trim().is_empty() {
            command.arg("-metadata").arg(format!("{key}={}", clean_tag(value)));
        }
    }
    if let Some(number) = metadata.track_number {
        command.arg("-metadata").arg(format!("track={number}"));
    }
    if let Some(year) = metadata.year {
        // Das Jahr steht im Datumsfeld, so wie es Musikprogramme erwarten.
        command.arg("-metadata").arg(format!("date={year}"));
        command.arg("-metadata").arg(format!("year={year}"));
    }
    let output = command
        .arg(&tagged_path)
        .output()
        .map_err(|e| format!("FFmpeg konnte die Datei nicht öffnen: {e}"))?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&tagged_path);
        let details = String::from_utf8_lossy(&output.stderr);
        let last = details.lines().last().unwrap_or("unbekannter Fehler");
        return Err(format!("Die Metadaten konnten nicht geschrieben werden: {last}"));
    }
    std::fs::copy(&tagged_path, media_path)
        .map_err(|e| format!("Die Datei konnte nicht ersetzt werden: {e}"))?;
    let _ = std::fs::remove_file(&tagged_path);
    // FFmpeg schreibt "lyrics" nur als TXXX-Rahmen. Ein echter USLT-Rahmen wird
    // hier ergänzt, damit auch iTunes und Windows den Text anzeigen.
    if let Some(text) = lyrics {
        write_lyrics(media_path, text)?;
    }
    Ok(())
}

/// Schreibt einen USLT-Rahmen (unsynchronisierter Songtext) in die ID3-Markierung.
///
/// Vorhandene Songtexte und der TXXX-Ersatz von FFmpeg werden dabei ersetzt.
fn write_lyrics(media_path: &Path, lyrics: &str) -> Result<(), String> {
    let ist_mp3 = media_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("mp3"))
        .unwrap_or(false);
    if !ist_mp3 {
        return Ok(());
    }
    let datei = std::fs::read(media_path).map_err(|e| e.to_string())?;

    // Markierung lesen. Ohne ID3-Kopf wird eine neue angelegt.
    let (version, vorhanden, audio_start) = if datei.len() >= 10 && &datei[0..3] == b"ID3" {
        let version = datei[3];
        let flags = datei[5];
        // Unsynchronisierte Markierungen werden nicht angefasst.
        if flags & 0x80 != 0 || (version != 3 && version != 4) {
            return Ok(());
        }
        let size = synchsafe(&datei[6..10]);
        let mut start = 10 + size;
        // Eine Fußzeile (nur v2.4) gehört mit zur Markierung.
        if flags & 0x10 != 0 {
            start += 10;
        }
        let start = start.min(datei.len());
        let body = datei[10..start].to_vec();
        (version, body, start)
    } else {
        (3u8, Vec::new(), 0)
    };

    let kopf_laenge = 10usize;
    let mut rahmen: Vec<u8> = Vec::new();
    let mut position = 0usize;
    while position + kopf_laenge <= vorhanden.len() {
        let id = &vorhanden[position..position + 4];
        if !id.iter().all(|byte| byte.is_ascii_alphanumeric()) {
            // Füllbytes am Ende der Markierung.
            break;
        }
        let rohe_laenge = &vorhanden[position + 4..position + 8];
        let laenge = if version == 4 {
            synchsafe(rohe_laenge)
        } else {
            u32::from_be_bytes([rohe_laenge[0], rohe_laenge[1], rohe_laenge[2], rohe_laenge[3]]) as usize
        };
        let gesamt = kopf_laenge + laenge;
        if position + gesamt > vorhanden.len() {
            break;
        }
        let daten = &vorhanden[position + kopf_laenge..position + gesamt];
        let ist_songtext = id == b"USLT";
        // Der TXXX-Rahmen von FFmpeg trägt die Beschreibung "USLT".
        let ist_ffmpeg_ersatz = id == b"TXXX"
            && daten.len() >= 5
            && daten[0] == 0
            && &daten[1..5] == b"USLT";
        if !ist_songtext && !ist_ffmpeg_ersatz {
            rahmen.extend_from_slice(&vorhanden[position..position + gesamt]);
        }
        position += gesamt;
    }

    let mut daten = Vec::with_capacity(lyrics.len() * 2 + 16);
    if version == 4 {
        daten.push(3); // UTF-8
        daten.extend_from_slice(b"eng");
        daten.push(0); // leere Beschreibung
        daten.extend_from_slice(lyrics.replace('\r', "").as_bytes());
    } else {
        daten.push(1); // UTF-16 mit BOM
        daten.extend_from_slice(b"eng");
        daten.extend_from_slice(&[0, 0]); // leere Beschreibung
        daten.extend_from_slice(&[0xff, 0xfe]);
        for einheit in lyrics.replace('\r', "").encode_utf16() {
            daten.extend_from_slice(&einheit.to_le_bytes());
        }
    }
    rahmen.extend_from_slice(b"USLT");
    if version == 4 {
        rahmen.extend_from_slice(&to_synchsafe(daten.len()));
    } else {
        rahmen.extend_from_slice(&(daten.len() as u32).to_be_bytes());
    }
    rahmen.extend_from_slice(&[0, 0]);
    rahmen.extend_from_slice(&daten);

    let mut neu = Vec::with_capacity(10 + rahmen.len() + datei.len() - audio_start);
    neu.extend_from_slice(b"ID3");
    neu.push(version);
    neu.push(0); // Revision
    neu.push(0); // keine Flags
    neu.extend_from_slice(&to_synchsafe(rahmen.len()));
    neu.extend_from_slice(&rahmen);
    neu.extend_from_slice(&datei[audio_start..]);

    std::fs::write(media_path, neu).map_err(|e| e.to_string())?;
    Ok(())
}

/// Liest eine synchsafe Zahl (7 Bit je Byte).
fn synchsafe(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .fold(0usize, |wert, byte| (wert << 7) | (byte & 0x7f) as usize)
}

/// Schreibt eine synchsafe Zahl als vier Bytes.
fn to_synchsafe(wert: usize) -> [u8; 4] {
    [
        ((wert >> 21) & 0x7f) as u8,
        ((wert >> 14) & 0x7f) as u8,
        ((wert >> 7) & 0x7f) as u8,
        (wert & 0x7f) as u8,
    ]
}

/// Entfernt Steuerzeichen aus einem Tag-Wert.
fn clean_tag(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .collect()
}
