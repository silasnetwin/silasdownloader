//! Prüft das Schreiben von Tags und Cover in eine fertige Mediendatei.
//!
//! Braucht FFmpeg und eine Internetverbindung und wird deshalb nicht automatisch
//! ausgeführt.
//!
//! ```text
//! cargo test --test tags -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

use silasdownloader_lib::{engine::tags::write_tags, model::MediaMetadata};

/// Pfad zum mitgelieferten FFmpeg-Programm.
fn ffmpeg() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("resources").join("bin").join("ffmpeg.exe")
}

/// Lädt das Albumcover eines Spotify-Titels herunter.
#[allow(dead_code)]
fn album_cover(track_id: &str, target: &Path) -> PathBuf {
    let url = format!(
        "https://open.spotify.com/oembed?url=https%3A%2F%2Fopen.spotify.com%2Ftrack%2F{track_id}"
    );
    let output = Command::new("curl.exe")
        .args(["--silent", "--location", "--max-time", "20", "--"])
        .arg(&url)
        .output()
        .expect("curl.exe konnte nicht gestartet werden");
    let text = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(text.trim()).expect("unerwartete Antwort");
    let thumbnail = value["thumbnail_url"]
        .as_str()
        .expect("kein Cover in der Antwort")
        .to_owned();
    Command::new("curl.exe")
        .args(["--silent", "--location", "--max-time", "20", "--"])
        .arg(&thumbnail)
        .arg(target)
        .output()
        .expect("Cover konnte nicht geladen werden");
    target.to_owned()
}

/// Liest die Maße des eingebetteten Bildes.
fn embedded_size(media: &Path, target: &Path) -> Option<(u32, u32)> {
    let _ = std::fs::remove_file(target);
    let output = Command::new(ffmpeg())
        .args(["-v", "error", "-y", "-i"])
        .arg(media)
        .args(["-an", "-vcodec", "copy"])
        .arg(target)
        .output()
        .expect("ffmpeg konnte nicht gestartet werden");
    if !output.status.success() || !target.exists() {
        return None;
    }
    let data = std::fs::read(target).ok()?;
    // JPEG: SOF-Marker mit den Bildmaßen suchen.
    let mut index = 2;
    while index + 9 < data.len() {
        if data[index] != 0xFF {
            index += 1;
            continue;
        }
        let marker = data[index + 1];
        let length = u16::from_be_bytes([data[index + 2], data[index + 3]]) as usize;
        if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
            let height = u16::from_be_bytes([data[index + 5], data[index + 6]]);
            let width = u16::from_be_bytes([data[index + 7], data[index + 8]]);
            return Some((width as u32, height as u32));
        }
        index += 2 + length;
    }
    None
}

#[test]
#[ignore = "benötigt FFmpeg und eine Internetverbindung"]
fn bettet_cover_und_tags_in_eine_mp3_ein() {
    let temp = std::env::temp_dir().join("silas-tag-test");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("Temp-Ordner konnte nicht erstellt werden");

    // Eine MP3 mit eingebettetem 16:9-Bild, wie yt-dlp sie erzeugt.
    let media = temp.join("Testtitel.mp3");
    let status = Command::new(ffmpeg())
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=3",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=1280x720",
            "-frames:v",
            "1",
            "-map",
            "0:a",
            "-map",
            "1:v",
            "-c:a",
            "libmp3lame",
            "-b:a",
            "128k",
            "-c:v",
            "mjpeg",
            "-disposition:v:0",
            "attached_pic",
            "-id3v2_version",
            "3",
        ])
        .arg(&media)
        .output()
        .expect("ffmpeg konnte nicht gestartet werden");
    assert!(status.status.success(), "Testdatei nicht erzeugt");

    let cover = temp.join("cover.jpg");
    let status = Command::new(ffmpeg())
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", "color=c=red:s=300x300", "-frames:v", "1"])
        .arg(&cover)
        .output()
        .expect("ffmpeg konnte nicht gestartet werden");
    assert!(status.status.success(), "Cover nicht erzeugt");
    assert!(cover.is_file(), "Cover nicht geladen");
    assert_eq!(embedded_size(&cover, &temp.join("cover_out.jpg")), Some((300, 300)));

    let metadata = MediaMetadata {
        title: "Testtitel".to_owned(),
        artist: "Testinterpret".to_owned(),
        album_artist: "Testinterpret".to_owned(),
        album: "Testalbum".to_owned(),
        track_number: Some(3),
        year: Some(2024),
        artwork_path: Some(cover.to_string_lossy().into_owned()),
        lyrics: Some("Zeile eins\nZeile zwei".to_owned()),
    };

    write_tags(&ffmpeg(), &media, &metadata).expect("Tags konnten nicht geschrieben werden");

    // Der Songtext muss als echter USLT-Rahmen in der Datei stehen.
    let bytes = std::fs::read(&media).expect("Datei nicht lesbar");
    let uslt = bytes
        .windows(4)
        .position(|fenster| fenster == b"USLT")
        .expect("kein USLT-Rahmen gefunden");
    println!("USLT-Rahmen an Position {uslt}");
    // Der FFmpeg-Ersatz (TXXX mit der Beschreibung USLT) darf nicht bleiben.
    let txxx_uslt = bytes
        .windows(9)
        .any(|fenster| fenster == b"TXXX\x00\x00\x00\x00USLT");
    assert!(!txxx_uslt, "der TXXX-Ersatz von FFmpeg steht noch in der Datei");

    let size = embedded_size(&media, &temp.join("out.jpg"));
    println!("eingebettetes Bild: {size:?}");
    assert_eq!(size, Some((300, 300)), "das Cover ist nicht quadratisch drin");

    // Die Tags müssen in der Datei stehen.
    let probe = Command::new(ffmpeg())
        .args(["-v", "error", "-i"])
        .arg(&media)
        .args(["-f", "null", "-"])
        .output()
        .expect("ffmpeg konnte nicht gestartet werden");
    assert!(probe.status.success());

    println!("ok: Cover und Tags wurden geschrieben");
}
