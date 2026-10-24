//! Liest öffentliche Spotify-Auswahlen und lädt sie als MP3.
//!
//! Die Logik stammt aus `raubkopien/spotify-metadata-exporter` und ist hier so
//! eingepasst, dass sie ohne zusätzliche Rust-Bibliotheken auskommt: die Seite wird
//! mit dem Windows-eigenen `curl.exe` geholt, das Parsen passiert auf serde_json.
//!
//! Es werden keine Zugangsdaten benötigt: Spotify rendert die Trackliste auch in der
//! öffentlichen Einbettungsseite. Private oder nicht eingebettete Inhalte sind damit
//! naturgemäß nicht abrufbar.

pub mod workflow;

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use serde_json::Value;

use crate::{
    engine::{
        naming::safe_filename,
        process::duration_is_close,
        tools::hidden_command,
    },
    model::{MediaMetadata, Settings},
};

/// Die parallele Anzahl der MP3-Jobs.
const MAX_PARALLEL_DOWNLOADS: usize = 4;

/// Die Art der Spotify-Auswahl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Track,
    Playlist,
    Album,
}

impl SourceType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Track => "track",
            Self::Playlist => "playlist",
            Self::Album => "album",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Track => "Track",
            Self::Playlist => "Playlist",
            Self::Album => "Album",
        }
    }
}

/// Ein Track aus der öffentlichen Spotify-Ansicht.
#[derive(Debug, Clone)]
pub struct SpotifyTrack {
    pub spotify_id: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album_artist: String,
    pub album: String,
    pub track_number: Option<u32>,
    /// Erscheinungsjahr des Albums.
    pub year: Option<u32>,
    /// Adresse des Albumcovers, wird beim Laden der Quelle heruntergeladen.
    pub artwork_url: Option<String>,
    /// Lokale Kopie des Albumcovers.
    pub artwork_path: Option<PathBuf>,
    /// Laufzeit in Sekunden.
    pub duration: Option<u64>,
}

/// Die gelesene Spotify-Auswahl.
#[derive(Debug)]
pub struct PublicSpotifySource {
    pub title: String,
    pub tracks: Vec<SpotifyTrack>,
    pub listed_tracks: usize,
}

/// Zerlegt einen Spotify-Link in Art und Kennung.
pub fn parse_spotify_url(input: &str) -> Result<(SourceType, String), String> {
    let trimmed = input.trim();
    let rest = trimmed
        .strip_prefix("https://open.spotify.com/")
        .or_else(|| trimmed.strip_prefix("http://open.spotify.com/"))
        .or_else(|| trimmed.strip_prefix("https://spotify.link/"))
        .ok_or_else(|| {
            "Ungültiger Spotify-Link. Erwartet wird https://open.spotify.com/track/..., \
             /playlist/... oder /album/..."
                .to_string()
        })?;
    let mut parts: Vec<&str> = rest.split('?').next().unwrap_or(rest).split('/').collect();
    // Spotify-Links können ein Sprachsegment wie /intl-de/ enthalten.
    if parts.first().is_some_and(|segment| segment.starts_with("intl-")) {
        parts.remove(0);
    }
    let parts: Vec<&str> = parts.into_iter().filter(|s| !s.is_empty()).collect();
    if parts.len() != 2 {
        return Err(
            "Der Spotify-Link muss genau einen Typ (track, playlist oder album) und eine ID enthalten."
                .to_string(),
        );
    }
    let source_type = match parts[0] {
        "track" => SourceType::Track,
        "playlist" => SourceType::Playlist,
        "album" => SourceType::Album,
        _ => {
            return Err(
                "Nicht unterstützter Spotify-Link. Erlaubt sind Track-, Playlist- und Album-Links."
                    .to_string(),
            )
        }
    };
    let id = parts[1];
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Die Spotify-ID im Link ist ungültig.".to_string());
    }
    Ok((source_type, id.to_owned()))
}

/// Holt eine Webseite als Bytes.
fn http_get(url: &str) -> Result<Vec<u8>, String> {
    let output = hidden_command("curl.exe")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg("30")
        .arg("--user-agent")
        .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/130 Safari/537.36")
        .arg("--")
        .arg(url)
        .output()
        .map_err(|e| format!("Die Seite konnte nicht geladen werden: {e}"))?;
    if !output.status.success() {
        return Err("Die Seite ist nicht erreichbar.".to_string());
    }
    Ok(output.stdout)
}

/// Holt die echten Albumcover der Tracks über die oEmbed-Schnittstelle von Spotify.
///
/// Die öffentliche Einbettungsseite nennt nur das Bild der Playlist oder des Albums.
/// Das Albumcover des einzelnen Titels steht dagegen in der oEmbed-Antwort. Es ist
/// quadratisch (300 × 300) und kommt direkt von Spotify.
fn fetch_album_covers(track_ids: &[String]) -> HashMap<String, String> {
    let mut covers = HashMap::new();
    // Spotify ist bei vielen Anfragen etwas empfindlich, daher in Blöcken.
    for chunk in track_ids.chunks(8) {
        let mut children = Vec::new();
        for id in chunk {
            let mut command = hidden_command("curl.exe");
            command
                .arg("--silent")
                .arg("--location")
                .arg("--max-time")
                .arg("20")
                .arg("--")
                .arg(album_cover_url(id))
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null());
            if let Ok(child) = command.spawn() {
                children.push((id.clone(), child));
            }
        }
        for (id, child) in children {
            let Ok(output) = child.wait_with_output() else {
                continue;
            };
            let text = String::from_utf8_lossy(&output.stdout);
            let Ok(value) = serde_json::from_str::<Value>(text.trim()) else {
                continue;
            };
            if let Some(thumbnail) = value.get("thumbnail_url").and_then(Value::as_str) {
                if !thumbnail.is_empty() {
                    covers.insert(id, thumbnail.to_owned());
                }
            }
        }
    }
    // Was fehlt, wird noch einmal allein versucht. Eine abgebrochene Verbindung
    // soll nicht den ganzen Titel ohne Cover lassen.
    for id in track_ids {
        if covers.contains_key(id) {
            continue;
        }
        let Ok(output) = hidden_command("curl.exe")
            .arg("--silent")
            .arg("--location")
            .arg("--max-time")
            .arg("20")
            .arg("--")
            .arg(album_cover_url(id))
            .output()
        else {
            continue;
        };
        let text = String::from_utf8_lossy(&output.stdout);
        let Ok(value) = serde_json::from_str::<Value>(text.trim()) else {
            continue;
        };
        if let Some(thumbnail) = value.get("thumbnail_url").and_then(Value::as_str) {
            if !thumbnail.is_empty() {
                covers.insert(id.clone(), thumbnail.to_owned());
            }
        }
    }
    covers
}

/// Die Adresse der Schnittstelle, die das Albumcover eines Titels nennt.
fn album_cover_url(track_id: &str) -> String {
    format!("https://open.spotify.com/oembed?url=https%3A%2F%2Fopen.spotify.com%2Ftrack%2F{track_id}")
}

/// Lädt die Coverbilder der Titel und hängt sie an die Einträge.
///
/// Die Bilder werden blockweise parallel geladen, damit eine ganze Playlist nicht
/// ewig dauert. Das Cover bekommt den Namen des Titels und liegt im übergebenen
/// (temporären) Ordner; in die MP3 wird es anschließend eingebettet.
fn load_covers(tracks: &mut [SpotifyTrack], artwork_dir: &Path, limit: usize) {
    // Erst die Ziele bestimmen. Gleiche Adressen teilen sich eine Datei.
    let mut targets: HashMap<String, PathBuf> = HashMap::new();
    let mut taken: HashSet<PathBuf> = HashSet::new();
    let mut order: Vec<(usize, String, PathBuf)> = Vec::new();
    for (index, track) in tracks.iter().enumerate().take(limit) {
        let Some(artwork) = track.artwork_url.clone() else {
            continue;
        };
        let path = match targets.get(&artwork) {
            Some(path) => path.clone(),
            None => {
                // Der Name folgt dem Titel. Zwei Titel mit gleichem Namen bekommen
                // deshalb eine laufende Nummer, damit sich die Cover nicht
                // gegenseitig überschreiben.
                let base = safe_filename(&track.title);
                let mut path = artwork_dir.join(format!("{base}.jpg"));
                let mut number = 2;
                while taken.contains(&path) {
                    path = artwork_dir.join(format!("{base}-{number}.jpg"));
                    number += 1;
                }
                taken.insert(path.clone());
                targets.insert(artwork.clone(), path.clone());
                path
            }
        };
        order.push((index, artwork, path));
    }

    for chunk in order.chunks(8) {
        let mut children = Vec::new();
        for (_, url, path) in chunk {
            let mut command = hidden_command("curl.exe");
            command
                .arg("--silent")
                .arg("--location")
                .arg("--max-time")
                .arg("25")
                .arg("--output")
                .arg(path)
                .arg("--")
                .arg(url)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            if let Ok(child) = command.spawn() {
                children.push(child);
            }
        }
        for mut child in children {
            let _ = child.wait();
        }
    }

    // Was fehlt, wird noch einmal allein versucht. Ein einzelner Fehlgriff vom
    // Server soll nicht den ganzen Track ohne Cover lassen.
    for (_, url, path) in &order {
        if path.is_file() {
            continue;
        }
        let _ = http_get(url).map(|bytes| std::fs::write(path, bytes));
    }

    // Jeder Titel bekommt sein eigenes Bild im temporären Ordner. Bei einem Album
    // ist das für alle Titel dasselbe Cover, aber so passt das Bild immer zum Titel.
    for (index, _, path) in order {
        if !path.is_file() {
            continue;
        }
        let own = {
            let base = safe_filename(&tracks[index].title);
            let mut own = artwork_dir.join(format!("{base}.jpg"));
            let mut number = 2;
            while taken.contains(&own) && own != path {
                own = artwork_dir.join(format!("{base}-{number}.jpg"));
                number += 1;
            }
            taken.insert(own.clone());
            own
        };
        if own != path {
            let _ = std::fs::copy(&path, &own);
        }
        if own.is_file() {
            tracks[index].artwork_path = Some(own);
        }
    }
}

/// Liest die öffentliche Trackliste einer Spotify-Auswahl.
///
/// `artwork_dir` bekommt die temporären Coverbilder. `cover_limit` begrenzt, wie
/// viele Albumcover geholt werden. Null bedeutet: gar keine.
pub fn fetch_public_source(
    url: &str,
    artwork_dir: &Path,
    cover_limit: usize,
) -> Result<PublicSpotifySource, String> {
    let (source_type, id) = parse_spotify_url(url)?;
    let page_url = format!("https://open.spotify.com/embed/{}/{}", source_type.as_str(), id);
    let html_bytes = http_get(&page_url)?;
    let html = String::from_utf8_lossy(&html_bytes).into_owned();

    let page_data = extract_next_data(&html)?;
    let mut tracks: Vec<SpotifyTrack> = Vec::new();
    let mut seen: HashMap<String, ()> = HashMap::new();
    // Die Angaben zum Album stehen am Objekt selbst. Die Meta-Tags der Seite sind
    // bei Spotify leer, deshalb wird das Objekt ausgewertet.
    let entity = find_entity(&page_data);
    let source_album = (source_type == SourceType::Album).then(|| AlbumContext {
        title: entity
            .and_then(|entity| entity.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        artists: entity
            .and_then(|entity| entity.get("subtitle"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        artwork_url: entity.and_then(find_artwork).map(str::to_owned),
        year: entity
            .and_then(|entity| entity.get("releaseDate"))
            .and_then(|date| date.get("isoString"))
            .and_then(Value::as_str)
            .and_then(parse_year),
    });
    collect_tracks(&page_data, &mut tracks, &mut seen, source_album);
    if tracks.is_empty() {
        return Err(
            "Spotify hat auf der öffentlichen Einbettungsseite keine Trackliste geliefert. \
             Für private oder nicht eingebettete Inhalte sind keine Daten verfügbar."
                .to_string(),
        );
    }
    // Album-Einbettungen nennen den Albumnamen oft nur am Albumobjekt. Die Reihenfolge
    // in genau dieser Ansicht ist zugleich die Tracknummer.
    if source_type == SourceType::Album {
        for (index, track) in tracks.iter_mut().enumerate() {
            if track.track_number.is_none() {
                track.track_number = Some(index as u32 + 1);
            }
        }
    }

    let listed_tracks = find_track_count(&page_data).unwrap_or(tracks.len());
    std::fs::create_dir_all(artwork_dir)
        .map_err(|e| format!("Temporärer Coverordner konnte nicht erstellt werden: {e}"))?;

    if cover_limit > 0 {
        // Das Albumcover des einzelnen Titels hat Vorrang vor dem Bild der Playlist.
        let album_covers = fetch_album_covers(
            &tracks
                .iter()
                .take(cover_limit)
                .map(|track| track.spotify_id.clone())
                .filter(|id| !id.is_empty())
                .collect::<Vec<_>>(),
        );
        for track in tracks.iter_mut().take(cover_limit) {
            if let Some(cover) = album_covers.get(&track.spotify_id) {
                track.artwork_url = Some(cover.clone());
            }
        }
        load_covers(&mut tracks, artwork_dir, cover_limit);
    }

    // Bei einem einzelnen Track nennt Spotify das Album nicht, bei Playlists fehlt
    // oft das Jahr. Die Angaben werden über Deezer (Rückfall: iTunes) ergänzt,
    // damit die MP3-Dateien im richtigen Album landen.
    fetch_album_details(&mut tracks, source_type == SourceType::Album);
    if source_type == SourceType::Album {
        fetch_album_year(&mut tracks);
    }

    let title = find_entity_name(&page_data)
        .or_else(|| page_title(&html))
        .unwrap_or_else(|| "Spotify-Auswahl".into());
    Ok(PublicSpotifySource {
        title,
        tracks,
        listed_tracks,
    })
}

/// Erzeugt die Suchanfrage für einen Track.
pub fn search_query(title: &str, artists: &[String]) -> String {
    let artist = artists.join(" ");
    if artist.is_empty() {
        title.to_owned()
    } else {
        format!("{artist} {title}")
    }
}

/// Ein YouTube-Treffer, der zu einem Spotify-Titel passt.
#[derive(Debug, Clone, PartialEq)]
pub struct YoutubeMatch {
    pub url: String,
    pub title: String,
    pub channel: String,
    pub duration: Option<u64>,
}

/// Wörter, die im Titel eines Treffers auf ein falsches Video hindeuten.
///
/// Die Liste wird nur angewendet, wenn das Wort nicht selbst im Spotify-Titel
/// oder im Interpreten vorkommt. Ein Song namens „Live Forever“ darf also
/// weiterhin als Live-Version erkannt werden.
const BAD_TITLE_WORDS: &[&str] = &[
    "was made",
    "making of",
    "how it was made",
    "how i made",
    "behind the scenes",
    "type beat",
    "type beats",
    "fl studio",
    "tutorial",
    "tutorials",
    "reaction",
    "reactions",
    "review",
    "explained",
    "interview",
    "podcast",
    "documentary",
    "full album",
    "whole album",
    "album stream",
    "mixtape",
    "compilation",
    "compilations",
    "playlist",
    "playlists",
    "medley",
    "megamix",
    "mashup",
    "mix",
    "mixes",
    "instrumental",
    "instrumentals",
    "karaoke",
    "cover",
    "covers",
    "remix",
    "remixes",
    "acoustic",
    "piano",
    "guitar",
    "orchestral",
    "8 bit",
    "8bit",
    "music box",
    "lullaby",
    "a cappella",
    "acapella",
    "vocals only",
    "backing track",
    "minus one",
    "tribute",
    "parody",
    "slowed",
    "reverb",
    "sped up",
    "speed up",
    "spedup",
    "nightcore",
    "daycore",
    "8d",
    "3d audio",
    "bass boost",
    "bass boosted",
    "432hz",
    "528hz",
    "963hz",
    "enhanced",
    "spatial audio",
    "live",
    "concert",
    "performance",
    "live session",
    "session",
    "extended",
    "radio edit",
    "teaser",
    "trailer",
    "snippet",
    "preview",
    "loop",
    "hour",
    "1 hour",
    "10 hours",
    "hour version",
    "unreleased",
    "leaked",
    "leak",
    "demo",
    "og",
    "remake",
    "fan made",
    "ai cover",
    "ai generated",
    "ai version",
    "but every",
    "but it s",
    "earrape",
    "ear rape",
    "intro",
    "outro",
    "interlude",
    "skit",
];

/// Wörter, die im Kanalnamen auf ein falsches Video hindeuten.
const BAD_CHANNEL_WORDS: &[&str] = &[
    "reaction",
    "reactions",
    "tutorial",
    "tutorials",
    "karaoke",
    "instrumental",
    "instrumentals",
    "cover",
    "covers",
    "remix",
    "remixes",
    "slowed",
    "reverb",
    "nightcore",
    "8d",
    "bass boost",
    "bass boosted",
    "playlist",
    "playlists",
    "compilation",
    "compilations",
    "mixtape",
    "mix",
    "mixes",
    "type beat",
    "type beats",
    "fl studio",
    "tribute",
    "parody",
];

/// Sucht zu einem Spotify-Titel das passende YouTube-Video.
///
/// `ytsearch1:` nimmt blind den ersten Treffer. Darunter sind bei Musik oft
/// „How it was made“-Videos, Stunden-Mixe, Cover oder Instrumentals. Deshalb
/// werden mehrere Treffer geholt und nach Titel, Interpret, Laufzeit und
/// offiziellem Kanal bewertet. Wenn kein Treffer eindeutig passt, lieber gar
/// nichts laden als die falsche Datei.
pub fn resolve_youtube_match(
    yt_dlp: &Path,
    deno: &Path,
    title: &str,
    artists: &[String],
    duration: Option<u64>,
) -> Result<YoutubeMatch, String> {
    let base = search_query(title, artists);
    let mut queries = vec![(base.clone(), 15usize)];
    for suffix in ["topic", "official audio"] {
        let query = format!("{base} {suffix}");
        if !queries.iter().any(|(known, _)| *known == query) {
            queries.push((query, 10));
        }
    }

    let mut last_error: Option<String> = None;
    for (query, limit) in queries {
        match search_youtube(yt_dlp, deno, &query, limit) {
            Ok(entries) => {
                if let Some(found) = pick_youtube_candidate(&entries, title, artists, duration) {
                    return Ok(found);
                }
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(match last_error {
        Some(error) => format!("Die Videosuche für „{title}“ ist fehlgeschlagen: {error}"),
        None => format!(
            "Für „{title}“ wurde kein passendes Video gefunden. Die Treffer waren Mixe, \
             Cover oder hatten eine andere Laufzeit."
        ),
    })
}

/// Holt die Suchtreffer von YouTube.
fn search_youtube(
    yt_dlp: &Path,
    deno: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<Value>, String> {
    let out = hidden_command(yt_dlp)
        .env("PYTHONIOENCODING", "utf-8")
        .arg("--ignore-config")
        .arg("--js-runtimes")
        .arg(format!("deno:{}", deno.display()))
        .arg("--flat-playlist")
        .arg("--dump-single-json")
        .arg("--no-warnings")
        .arg("--no-progress")
        .arg("--socket-timeout")
        .arg("15")
        .arg("--retries")
        .arg("2")
        .arg(format!("ytsearch{limit}:{query}"))
        .output()
        .map_err(|error| format!("yt-dlp konnte nicht gestartet werden: {error}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let last = stderr
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("unbekannter Fehler")
            .trim()
            .to_owned();
        return Err(last);
    }
    let value: Value = serde_json::from_slice(&out.stdout)
        .map_err(|error| format!("Die Suchergebnisse konnten nicht gelesen werden: {error}"))?;
    Ok(value
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

/// Wählt aus den Suchtreffern das plausibelste Video aus.
pub fn pick_youtube_candidate(
    entries: &[Value],
    title: &str,
    artists: &[String],
    expected_duration: Option<u64>,
) -> Option<YoutubeMatch> {
    let expected_duration = expected_duration.filter(|duration| *duration > 0);
    let track_title = {
        let stripped = normalize_words(&strip_title_additions(title));
        if stripped.is_empty() {
            normalize_words(title)
        } else {
            stripped
        }
    };
    if track_title.is_empty() {
        return None;
    }
    let track_text = normalize_words(&format!("{title} {}", artists.join(" ")));
    let artist_names: Vec<String> = artists
        .iter()
        .map(|artist| normalize_words(artist))
        .filter(|artist| !artist.is_empty() && artist != "various artists")
        .collect();

    let mut best: Option<(i32, YoutubeMatch)> = None;
    for entry in entries {
        if entry
            .get("live_status")
            .and_then(Value::as_str)
            .is_some_and(|status| status == "is_live" || status == "is_upcoming")
        {
            continue;
        }
        let Some(id) = entry.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(candidate_title) = entry.get("title").and_then(Value::as_str) else {
            continue;
        };
        if id.is_empty() || candidate_title.trim().is_empty() {
            continue;
        }
        let channel = entry
            .get("channel")
            .and_then(Value::as_str)
            .or_else(|| entry.get("uploader").and_then(Value::as_str))
            .unwrap_or_default();
        let duration = entry.get("duration").and_then(number);
        let title_norm = normalize_words(candidate_title);
        let clean_norm = normalize_words(&strip_brackets(candidate_title));
        let channel_norm = normalize_words(channel);

        if contains_unwanted(&title_norm, &track_text, BAD_TITLE_WORDS)
            || contains_unwanted(&channel_norm, &track_text, BAD_CHANNEL_WORDS)
        {
            continue;
        }
        if let (Some(expected), Some(actual)) = (expected_duration, duration) {
            if !duration_is_close(expected, actual as f64) {
                continue;
            }
        }
        let Some(title_points) =
            title_match_points(&title_norm, &clean_norm, candidate_title, &track_title)
        else {
            continue;
        };
        let artist_points = artist_match_points(&title_norm, &channel_norm, &artist_names);
        if !artist_names.is_empty() && artist_points == 0 {
            continue;
        }
        let official_points = official_points(
            &title_norm,
            &channel_norm,
            entry
                .get("channel_is_verified")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        );
        let duration_points = match (expected_duration, duration) {
            (Some(expected), Some(actual)) => {
                let tolerance = (expected as f64 * 0.2).max(15.0).min(60.0);
                let difference = (actual as f64 - expected as f64).abs();
                (40.0 * (1.0 - difference / tolerance)).round() as i32
            }
            _ => 0,
        };
        let score = title_points + artist_points + official_points + duration_points;
        if score < 80 {
            continue;
        }

        let url = entry
            .get("url")
            .and_then(Value::as_str)
            .filter(|url| url.starts_with("https://"))
            .map(str::to_owned)
            .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={id}"));
        let candidate = YoutubeMatch {
            url,
            title: candidate_title.to_owned(),
            channel: channel.to_owned(),
            duration,
        };
        if best
            .as_ref()
            .map(|(best_score, _)| score > *best_score)
            .unwrap_or(true)
        {
            best = Some((score, candidate));
        }
    }

    best.map(|(_, candidate)| candidate)
}

/// Liest eine Zahl aus einem JSON-Wert, auch wenn sie als Fließkommazahl kommt.
fn number(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_f64().filter(|number| *number > 0.0).map(|number| number.round() as u64))
}

/// Entfernt Klammerzusätze wie „(Official Audio)“ oder „[Lyrics]“.
fn strip_brackets(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut depth = 0usize;
    for character in value.chars() {
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(character),
            _ => {}
        }
    }
    out
}

/// Prüft, ob eine Wortgruppe als eigenständige Wörter im Text vorkommt.
fn contains_phrase(text: &str, phrase: &str) -> bool {
    if phrase.is_empty() {
        return false;
    }
    format!(" {text} ").contains(&format!(" {phrase} "))
}

/// Prüft, ob ein Treffer ein unerwünschtes Wort enthält, das nicht zum Track gehört.
fn contains_unwanted(candidate: &str, track: &str, words: &[&str]) -> bool {
    words
        .iter()
        .any(|word| contains_phrase(candidate, word) && !contains_phrase(track, word))
}

/// Zerlegt einen Videotitel an den üblichen Trennern in Titel und Interpret.
fn title_segments(title: &str) -> Vec<String> {
    let mut parts: Vec<&str> = vec![title];
    for separator in [" - ", " – ", " — ", " ~ ", " | ", ":"] {
        let mut next = Vec::new();
        for part in parts {
            next.extend(part.split(separator));
        }
        parts = next;
    }
    parts
        .into_iter()
        .map(|part| normalize_words(&strip_brackets(part)))
        .filter(|part| !part.is_empty())
        .collect()
}

/// Bewertet, wie gut der Videotitel zum Track passt.
fn title_match_points(
    title_norm: &str,
    clean_norm: &str,
    raw_title: &str,
    track_title: &str,
) -> Option<i32> {
    if track_title.is_empty() {
        return None;
    }
    if clean_norm == track_title
        || title_segments(raw_title)
            .iter()
            .any(|segment| segment == track_title)
    {
        return Some(75);
    }

    // Bei sehr kurzen oder allgemeinen Titeln wäre eine Teilübereinstimmung
    // zu unsicher („Intro“ passt sonst auf jedes „Mufasa Intro“).
    let words = track_title.split_whitespace().count();
    let characters = track_title.chars().filter(|c| c.is_alphanumeric()).count();
    let generic = matches!(track_title, "intro" | "outro" | "interlude" | "skit");
    if generic || (words == 1 && characters <= 5) {
        return None;
    }

    if contains_phrase(clean_norm, track_title) || contains_phrase(title_norm, track_title) {
        Some(45)
    } else {
        None
    }
}

/// Bewertet, ob der Interpret im Titel oder im Kanalnamen vorkommt.
fn artist_match_points(title_norm: &str, channel_norm: &str, artists: &[String]) -> i32 {
    let title_compact = title_norm.replace(' ', "");
    let channel_compact = channel_norm.replace(' ', "");
    let mut points = 0;
    for artist in artists {
        let compact = artist.replace(' ', "");
        let channel_exact = channel_norm == *artist
            || channel_norm.ends_with(&format!(" {artist}"))
            || channel_compact == compact;
        if channel_exact {
            points = points.max(50);
        } else if compact.len() >= 3
            && (channel_compact.contains(&compact) || channel_norm.contains(artist.as_str()))
        {
            points = points.max(35);
        }
        if contains_phrase(title_norm, artist)
            || (compact.len() >= 3 && title_compact.contains(&compact))
        {
            points = points.max(30);
        }
    }
    points
}

/// Vergibt Zusatzpunkte für offizielle Kanäle und Kennzeichnungen.
fn official_points(title_norm: &str, channel_norm: &str, verified: bool) -> i32 {
    let mut points = 0;
    if channel_norm.ends_with(" topic") {
        points += 45;
    }
    if channel_norm.contains("vevo") {
        points += 25;
    }
    if verified {
        points += 30;
    }
    if channel_norm.contains("official") {
        points += 10;
    }
    if contains_phrase(title_norm, "official audio") {
        points += 20;
    } else if contains_phrase(title_norm, "official music video")
        || contains_phrase(title_norm, "official video")
    {
        points += 10;
    }
    if contains_phrase(title_norm, "audio") {
        points += 5;
    }
    if contains_phrase(title_norm, "lyrics") {
        points += 5;
    }
    points
}

/// Wandelt einen Track in die Metadaten für die Engine um.
pub fn to_metadata(track: &SpotifyTrack) -> MediaMetadata {
    MediaMetadata {
        title: track.title.clone(),
        artist: track.artists.join(", "),
        album_artist: track.album_artist.clone(),
        album: track.album.clone(),
        track_number: track.track_number,
        year: track.year,
        artwork_path: track
            .artwork_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        lyrics: None,
    }
}

/// Baut die Engine-Einstellungen für einen MP3-Download.
pub fn audio_settings(output_dir: &Path, quality: u32, embed_thumbnail: bool) -> Settings {
    let mut settings = Settings::default();
    settings.mode = crate::model::DownloadMode::Audio;
    settings.audio_quality = quality.clamp(64, 320);
    settings.output_dir = output_dir.to_string_lossy().into_owned();
    settings.embed_metadata = true;
    settings.embed_thumbnail = embed_thumbnail;
    settings.separate_folder_per_video = false;
    settings.filename_template = "{title}".into();
    settings.retries = 3;
    settings.concurrent_fragments = 8;
    settings
}

/// Die Anzahl der gleichzeitig laufenden MP3-Jobs.
pub fn parallel_jobs(total: usize) -> usize {
    MAX_PARALLEL_DOWNLOADS.min(total.max(1))
}

#[derive(Debug, Clone, Default)]
struct AlbumContext {
    /// Der Albumname. Bei Playlists gibt es keinen, dann bleibt er leer.
    title: String,
    artists: String,
    artwork_url: Option<String>,
    /// Erscheinungsjahr des Albums.
    year: Option<u32>,
}

/// Ein Titel, für den Albumangaben fehlen.
struct AlbumLookup {
    index: usize,
    artist: String,
    title: String,
    /// Laufzeit des Spotify-Titels; hilft, Remixe und Karaoke-Fassungen zu erkennen.
    duration: Option<u64>,
}

/// Holt die Albumangaben für Titel, bei denen Spotify sie nicht nennt.
///
/// Die öffentliche Einbettung eines einzelnen Tracks nennt weder Album noch
/// Tracknummer. Bei Playlists fehlt oft das Jahr. Deshalb werden die Angaben über
/// die öffentliche Deezer-Suche ergänzt; iTunes dient als Rückfall. Damit daraus
/// kein falsches Album wird, werden Interpret und Titel genau verglichen und
/// Treffer von fremden Interpreten verworfen.
fn fetch_album_details(tracks: &mut [SpotifyTrack], album_source: bool) {
    // Titel ohne Album brauchen Album, Nummer und Jahr. Titel mit Album, aber ohne
    // Jahr, brauchen nur das Jahr. Bei einer Album-Auswahl gilt das Jahr für alle
    // Titel gemeinsam und wird weiter unten einmal ermittelt.
    let wanted: Vec<AlbumLookup> = tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| {
            track.album.trim().is_empty() || (!album_source && track.year.is_none())
        })
        .map(|(index, track)| AlbumLookup {
            index,
            artist: track.artists.join(", "),
            title: strip_title_additions(&track.title),
            duration: track.duration,
        })
        .collect();
    if wanted.is_empty() {
        return;
    }

    // Deezer kennt auch neue Titel, bei denen die iTunes-Suche oft nichts findet.
    fetch_deezer_details(tracks, &wanted);

    // Alles, was Deezer nicht kannte, versucht es noch einmal bei iTunes.
    let remaining: Vec<&AlbumLookup> = wanted
        .iter()
        .filter(|lookup| {
            let track = &tracks[lookup.index];
            track.album.trim().is_empty() || track.year.is_none()
        })
        .collect();
    fetch_itunes_details(tracks, &remaining);
}

/// Füllt die Angaben eines Treffers in einen Track, ohne vorhandenes zu überschreiben.
fn apply_album_match(track: &mut SpotifyTrack, details: AlbumMatch) {
    if track.album.trim().is_empty() && !details.album.trim().is_empty() {
        track.album = details.album;
    }
    if track.album_artist.trim().is_empty() && !details.album_artist.trim().is_empty() {
        track.album_artist = details.album_artist;
    }
    if track.track_number.is_none() {
        track.track_number = details.track_number;
    }
    if track.year.is_none() {
        track.year = details.year;
    }
}

/// Der Treffer aus der Deezer-Suche.
struct DeezerMatch {
    track_id: u64,
    album: String,
    album_artist: String,
}

/// Sucht die fehlenden Angaben über die öffentliche Deezer-Schnittstelle.
fn fetch_deezer_details(tracks: &mut [SpotifyTrack], wanted: &[AlbumLookup]) {
    let mut hits: Vec<(usize, DeezerMatch)> = Vec::new();
    for chunk in wanted.chunks(8) {
        let mut children = Vec::new();
        for lookup in chunk {
            let term = format!("{} {}", lookup.artist, lookup.title);
            let url = format!(
                "https://api.deezer.com/search?q={}&limit=10",
                encode_query(&term)
            );
            let mut command = hidden_command("curl.exe");
            command
                .arg("--silent")
                .arg("--location")
                .arg("--max-time")
                .arg("20")
                .arg("--")
                .arg(&url)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null());
            if let Ok(child) = command.spawn() {
                children.push((lookup, child));
            }
        }
        for (lookup, child) in children {
            let Ok(output) = child.wait_with_output() else {
                continue;
            };
            let text = String::from_utf8_lossy(&output.stdout);
            let Ok(value) = serde_json::from_str::<Value>(text.trim()) else {
                continue;
            };
            if let Some(found) =
                pick_deezer_match(&value, &lookup.artist, &lookup.title, lookup.duration)
            {
                hits.push((lookup.index, found));
            }
        }
        // Deezer erlaubt etwa 50 Anfragen in fünf Sekunden. Bei großen Listen
        // würden die Blöcke sonst zu schnell aufeinander folgen und die Suche
        // liefert nur noch Fehler.
        if wanted.len() > 8 {
            thread::sleep(Duration::from_millis(700));
        }
    }
    if hits.is_empty() {
        return;
    }

    // Die Suche nennt weder Tracknummer noch Erscheinungsjahr. Beides steht im
    // einzelnen Track, den wir deshalb noch einmal abrufen.
    let mut details: Vec<(usize, AlbumMatch)> = Vec::new();
    for chunk in hits.chunks(8) {
        let mut children = Vec::new();
        for (index, found) in chunk {
            let url = format!("https://api.deezer.com/track/{}", found.track_id);
            let mut command = hidden_command("curl.exe");
            command
                .arg("--silent")
                .arg("--location")
                .arg("--max-time")
                .arg("20")
                .arg("--")
                .arg(&url)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null());
            if let Ok(child) = command.spawn() {
                children.push((*index, found, child));
            }
        }
        for (index, found, child) in children {
            let Ok(output) = child.wait_with_output() else {
                continue;
            };
            let text = String::from_utf8_lossy(&output.stdout);
            let value = serde_json::from_str::<Value>(text.trim()).ok();
            let mut details_for_track =
                value
                    .as_ref()
                    .and_then(pick_deezer_track)
                    .unwrap_or(AlbumMatch {
                    album: found.album.clone(),
                    album_artist: found.album_artist.clone(),
                    track_number: None,
                    year: None,
                });
            // Fehlt im Detail etwas, gilt der Treffer aus der Suche.
            if details_for_track.album.trim().is_empty() {
                details_for_track.album = found.album.clone();
            }
            if details_for_track.album_artist.trim().is_empty() {
                details_for_track.album_artist = found.album_artist.clone();
            }
            details.push((index, details_for_track));
        }
        if hits.len() > 8 {
            thread::sleep(Duration::from_millis(700));
        }
    }
    for (index, details) in details {
        apply_album_match(&mut tracks[index], details);
    }
}

/// Sucht den passenden Treffer aus der Antwort der Deezer-Suche.
///
/// Bevorzugt wird eine Fassung mit passender Laufzeit und ein richtiges Album
/// gegenüber einer Single. So fallen Remixe und Karaoke-Aufnahmen heraus.
fn pick_deezer_match(
    value: &Value,
    artist: &str,
    title: &str,
    duration: Option<u64>,
) -> Option<DeezerMatch> {
    let results = value.get("data")?.as_array()?;
    let mut best: Option<(u32, DeezerMatch)> = None;
    for result in results {
        let Some(result_title) = result.get("title").and_then(Value::as_str) else {
            continue;
        };
        let Some(result_artist) = result
            .get("artist")
            .and_then(|artist| artist.get("name"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        if !same_words(result_artist, artist)
            || !same_words(&strip_title_additions(result_title), title)
        {
            continue;
        }
        let Some(album) = result
            .get("album")
            .and_then(|album| album.get("title"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        if album.trim().is_empty() {
            continue;
        }
        let Some(track_id) = result.get("id").and_then(Value::as_u64) else {
            continue;
        };
        let result_duration = result.get("duration").and_then(Value::as_u64);
        let close = match (duration, result_duration) {
            (Some(expected), Some(actual)) => expected.abs_diff(actual) <= 2,
            _ => false,
        };
        // Ein exakter Titel schlägt eine Fassung mit Zusatz wie "(stripped)".
        let exact_title = normalize_words(result_title) == normalize_words(title);
        let is_single = album.to_lowercase().contains("single")
            || album
                .trim()
                .eq_ignore_ascii_case(strip_title_additions(result_title).trim());
        // Kleiner ist besser: exakter Titel, passende Laufzeit, ganzes Album.
        let score = if exact_title { 0 } else { 100 }
            + if close { 0 } else { 10 }
            + if is_single { 1 } else { 0 };
        let better = match &best {
            None => true,
            Some((best_score, _)) => score < *best_score,
        };
        if better {
            best = Some((
                score,
                DeezerMatch {
                    track_id,
                    album: album.to_owned(),
                    album_artist: result_artist.to_owned(),
                },
            ));
        }
    }
    best.map(|(_, found)| found)
}

/// Liest die vollständigen Angaben aus der Antwort von `/track/{id}`.
fn pick_deezer_track(value: &Value) -> Option<AlbumMatch> {
    let album = value
        .get("album")
        .and_then(|album| album.get("title"))
        .and_then(Value::as_str)?;
    if album.trim().is_empty() {
        return None;
    }
    let album_artist = value
        .get("artist")
        .and_then(|artist| artist.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let track_number = value
        .get("track_position")
        .and_then(Value::as_u64)
        .and_then(|number| u32::try_from(number).ok());
    let year = value
        .get("release_date")
        .and_then(Value::as_str)
        .and_then(parse_year);
    Some(AlbumMatch {
        album: album.to_owned(),
        album_artist: album_artist.to_owned(),
        track_number,
        year,
    })
}

/// Liest eine JSON-Antwort über HTTP.
fn http_json(url: &str) -> Option<Value> {
    let bytes = http_get(url).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    serde_json::from_str(text.trim()).ok()
}

/// Ermittelt das Erscheinungsjahr einer Album-Auswahl über Deezer.
///
/// Die Einbettungsseite eines Albums liefert kein Erscheinungsdatum. Statt für
/// jeden Titel einzeln zu suchen, wird das Jahr einmal für das ganze Album geholt.
fn fetch_album_year(tracks: &mut [SpotifyTrack]) {
    let Some(first) = tracks.first() else {
        return;
    };
    if first.album.trim().is_empty() || first.album_artist.trim().is_empty() {
        return;
    }
    if tracks.iter().all(|track| track.year.is_some()) {
        return;
    }
    let term = format!("{} {}", first.album, first.album_artist);
    let url = format!(
        "https://api.deezer.com/search/album?q={}&limit=10",
        encode_query(&term)
    );
    let Some(value) = http_json(&url) else {
        return;
    };
    let Some(album_id) = pick_deezer_album(&value, &first.album, &first.album_artist) else {
        return;
    };
    let Some(value) = http_json(&format!("https://api.deezer.com/album/{album_id}")) else {
        return;
    };
    let Some(year) = value
        .get("release_date")
        .and_then(Value::as_str)
        .and_then(parse_year)
    else {
        return;
    };
    for track in tracks.iter_mut() {
        if track.year.is_none() {
            track.year = Some(year);
        }
    }
}

/// Sucht das passende Album aus der Antwort der Deezer-Albumsuche.
fn pick_deezer_album(value: &Value, title: &str, artist: &str) -> Option<u64> {
    let results = value.get("data")?.as_array()?;
    let mut best = None;
    for result in results {
        let Some(name) = result.get("title").and_then(Value::as_str) else {
            continue;
        };
        let Some(result_artist) = result
            .get("artist")
            .and_then(|artist| artist.get("name"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        if !same_words(name, title) || !same_words(result_artist, artist) {
            continue;
        }
        let Some(id) = result.get("id").and_then(Value::as_u64) else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case(title.trim()) {
            return Some(id);
        }
        best.get_or_insert(id);
    }
    best
}

/// Ergänzt fehlende Angaben über die iTunes-Suche.
fn fetch_itunes_details(tracks: &mut [SpotifyTrack], wanted: &[&AlbumLookup]) {
    if wanted.is_empty() {
        return;
    }
    for chunk in wanted.chunks(8) {
        let mut children = Vec::new();
        for lookup in chunk {
            let term = format!("{} {}", lookup.artist, lookup.title);
            let url = format!(
                "https://itunes.apple.com/search?term={}&entity=song&limit=10",
                encode_query(&term)
            );
            let mut command = hidden_command("curl.exe");
            command
                .arg("--silent")
                .arg("--location")
                .arg("--max-time")
                .arg("20")
                .arg("--")
                .arg(&url)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null());
            if let Ok(child) = command.spawn() {
                children.push((lookup, child));
            }
        }
        for (lookup, child) in children {
            let Ok(output) = child.wait_with_output() else {
                continue;
            };
            let text = String::from_utf8_lossy(&output.stdout);
            let Ok(value) = serde_json::from_str::<Value>(text.trim()) else {
                continue;
            };
            if let Some(details) = pick_album_match(&value, &lookup.artist, &lookup.title) {
                apply_album_match(&mut tracks[lookup.index], details);
            }
        }
    }
}

/// Die Albumangaben eines Treffers.
struct AlbumMatch {
    album: String,
    album_artist: String,
    track_number: Option<u32>,
    year: Option<u32>,
}

/// Sucht den passenden Treffer aus der Antwort der iTunes-Suche.
///
/// Ein Treffer zählt nur, wenn Interpret und Titel übereinstimmen. Bevorzugt wird
/// das richtige Album gegenüber einer Single.
fn pick_album_match(value: &Value, artist: &str, title: &str) -> Option<AlbumMatch> {
    let results = value.get("results")?.as_array()?;
    let mut album_match: Option<AlbumMatch> = None;
    let mut single_match: Option<AlbumMatch> = None;
    for result in results {
        let Some(result_artist) = result.get("artistName").and_then(Value::as_str) else {
            continue;
        };
        let Some(result_title) = result.get("trackName").and_then(Value::as_str) else {
            continue;
        };
        if !same_words(result_artist, artist) || !same_words(result_title, title) {
            continue;
        }
        let Some(album) = result.get("collectionName").and_then(Value::as_str) else {
            continue;
        };
        if album.trim().is_empty() {
            continue;
        }
        let track_number = result
            .get("trackNumber")
            .and_then(Value::as_u64)
            .and_then(|number| u32::try_from(number).ok());
        let year = result
            .get("releaseDate")
            .and_then(Value::as_str)
            .and_then(|date| date.get(..4))
            .and_then(|year| year.parse::<u32>().ok());
        let details = AlbumMatch {
            album: album.to_owned(),
            album_artist: result_artist.to_owned(),
            track_number,
            year,
        };
        let is_single = album.to_lowercase().contains("single")
            || result
                .get("trackCount")
                .and_then(Value::as_u64)
                .is_some_and(|count| count <= 2);
        if is_single {
            single_match.get_or_insert(details);
        } else {
            album_match.get_or_insert(details);
        }
    }
    album_match.or(single_match)
}

/// Vergleicht zwei Namen, ohne Kleinschreibung, Satzzeichen und Zusätze.
///
/// "Ariana Grande" und "Ariana Grande & "PASSION"" gelten als gleich, wenn alle
/// Wörter der einen Angabe in der anderen stehen.
fn same_words(left: &str, right: &str) -> bool {
    let left = normalize_words(left);
    let right = normalize_words(right);
    if left.is_empty() || right.is_empty() {
        return false;
    }
    let left_parts: Vec<&str> = left.split(' ').collect();
    let right_parts: Vec<&str> = right.split(' ').collect();
    let (shorter, longer) = if left_parts.len() <= right_parts.len() {
        (&left_parts, &right_parts)
    } else {
        (&right_parts, &left_parts)
    };
    shorter.iter().all(|word| longer.contains(word))
}

/// Bringt einen Namen in eine vergleichbare Form.
fn normalize_words(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut last_space = true;
    for character in value.chars() {
        if character.is_alphanumeric() {
            for lower in character.to_lowercase() {
                out.push(lower);
            }
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim().to_owned()
}

/// Entfernt Zusätze wie "(feat. ...)" oder "- Remastered" vom Titel.
fn strip_title_additions(title: &str) -> String {
    let mut base = title.trim();
    while let Some(index) = base.find('(') {
        if base[index..].contains(')') {
            base = base[..index].trim();
        } else {
            break;
        }
    }
    if let Some(index) = base.find(" - ") {
        base = &base[..index];
    }
    base.trim().to_owned()
}

/// Wandelt einen Suchbegriff in eine URL-taugliche Form.
fn encode_query(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

fn extract_next_data(html: &str) -> Result<Value, String> {
    let marker = "__NEXT_DATA__";
    let marker_at = html
        .find(marker)
        .ok_or("Die Spotify-Seite enthält keine erwarteten eingebetteten Seitendaten")?;
    let script_start = html[..marker_at]
        .rfind("<script")
        .ok_or("Die Spotify-Seitendaten konnten nicht lokalisiert werden")?;
    let content_start = html[marker_at..]
        .find('>')
        .map(|offset| marker_at + offset + 1)
        .ok_or("Die Spotify-Seitendaten haben kein gültiges Script-Element")?;
    let content_end = html[content_start..]
        .find("</script>")
        .map(|offset| content_start + offset)
        .ok_or("Die Spotify-Seitendaten sind unvollständig")?;
    if script_start >= content_start || content_end <= content_start {
        return Err("Die Spotify-Seitendaten sind ungültig".into());
    }
    serde_json::from_str(&html[content_start..content_end])
        .map_err(|e| format!("Die Spotify-Seitendaten sind kein gültiges JSON: {e}"))
}

fn collect_tracks(
    value: &Value,
    out: &mut Vec<SpotifyTrack>,
    seen: &mut HashMap<String, ()>,
    inherited_album: Option<AlbumContext>,
) {
    match value {
        Value::Object(object) => {
            // Der Kontext kommt von einem Album oder einer Playlist. Beide tragen
            // das Cover; nur ein Album liefert auch den Albumnamen.
            let album_context = object
                .get("uri")
                .and_then(Value::as_str)
                .filter(|uri| {
                    uri.starts_with("spotify:album:") || uri.starts_with("spotify:playlist:")
                })
                .or_else(|| {
                    object
                        .get("type")
                        .or_else(|| object.get("__typename"))
                        .and_then(Value::as_str)
                        .filter(|kind| {
                            kind.eq_ignore_ascii_case("album")
                                || kind.eq_ignore_ascii_case("playlist")
                        })
                        .map(|_| "spotify:playlist:")
                })
                .and_then(|_| object.get("name").and_then(Value::as_str))
                .map(|name| {
                    let is_album = object
                        .get("uri")
                        .and_then(Value::as_str)
                        .is_some_and(|uri| uri.starts_with("spotify:album:"))
                        || object
                            .get("type")
                            .or_else(|| object.get("__typename"))
                            .and_then(Value::as_str)
                            .is_some_and(|kind| kind.eq_ignore_ascii_case("album"));
                    AlbumContext {
                        title: if is_album { name.to_owned() } else { String::new() },
                        artists: object
                            .get("artists")
                            .and_then(Value::as_array)
                            .map(|items| {
                                items
                                    .iter()
                                    .filter_map(|item| item.get("name").and_then(Value::as_str))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .filter(|value| !value.is_empty())
                            // Bei einem Album steht der Interpret im Untertitel. Bei
                            // einer Playlist steht dort der Ersteller - der ist kein
                            // Album-Interpret.
                            .or_else(|| {
                                is_album
                                    .then(|| {
                                        object
                                            .get("subtitle")
                                            .and_then(Value::as_str)
                                            .map(str::to_owned)
                                    })
                                    .flatten()
                            })
                            .unwrap_or_default(),
                        artwork_url: object
                            .get("coverArt")
                            .or_else(|| object.get("images"))
                            .and_then(find_artwork)
                            .or_else(|| find_artwork(value))
                            .map(str::to_owned),
                        year: object
                            .get("releaseDate")
                            .and_then(|date| date.get("isoString"))
                            .and_then(Value::as_str)
                            .and_then(parse_year),
                    }
                })
                .or(inherited_album);
            if let (Some(uri), Some(title)) = (
                object.get("uri").and_then(Value::as_str),
                object
                    .get("name")
                    .or_else(|| object.get("title"))
                    .and_then(Value::as_str),
            ) {
                if let Some(id) = uri.strip_prefix("spotify:track:") {
                    if !id.is_empty() && !title.trim().is_empty() && !seen.contains_key(id) {
                        let album_value = object.get("albumOfTrack").or_else(|| object.get("album"));
                        let track_number = object
                            .get("trackNumber")
                            .or_else(|| object.get("track_number"))
                            .or_else(|| album_value.and_then(|album| album.get("trackNumber")))
                            .or_else(|| album_value.and_then(|album| album.get("track_number")))
                            .and_then(parse_number);
                        let artists = object
                            .get("artists")
                            .and_then(Value::as_array)
                            .map(|items| {
                                items
                                    .iter()
                                    .filter_map(|item| {
                                        item.get("name").and_then(Value::as_str).map(str::to_owned)
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .filter(|items| !items.is_empty())
                            .or_else(|| {
                                object.get("subtitle").and_then(Value::as_str).map(|s| {
                                    s.split(',')
                                        .map(str::trim)
                                        .filter(|s| !s.is_empty())
                                        .map(str::to_owned)
                                        .collect()
                                })
                            })
                            .unwrap_or_default();
                        let album = album_value
                            .and_then(|value| {
                                value
                                    .as_str()
                                    .or_else(|| value.get("name").and_then(Value::as_str))
                            })
                            .or_else(|| object.get("albumName").and_then(Value::as_str))
                            .map(str::to_owned)
                            .or_else(|| album_context.as_ref().map(|album| album.title.clone()))
                            .unwrap_or_default();
                        let album_artist = album_value
                            .and_then(|album| album.get("artists"))
                            .and_then(Value::as_array)
                            .map(|items| {
                                items
                                    .iter()
                                    .filter_map(|item| item.get("name").and_then(Value::as_str))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .filter(|value| !value.is_empty())
                            .or_else(|| {
                                album_context
                                    .as_ref()
                                    .filter(|album| !album.artists.is_empty())
                                    .map(|album| album.artists.clone())
                            })
                            .unwrap_or_else(|| artists.first().cloned().unwrap_or_default());
                        let artwork_url = album_value
                            .and_then(find_artwork)
                            .or_else(|| object.get("coverArt").and_then(find_artwork))
                            .or_else(|| object.get("images").and_then(find_artwork))
                            .or_else(|| {
                                album_context
                                    .as_ref()
                                    .and_then(|album| album.artwork_url.as_deref())
                            })
                            .map(str::to_owned);
                        let duration = object
                            .get("duration")
                            .and_then(Value::as_u64)
                            .map(|milliseconds| milliseconds / 1000);
                        out.push(SpotifyTrack {
                            spotify_id: id.to_owned(),
                            title: title.to_owned(),
                            artists,
                            album_artist,
                            album,
                            track_number,
                            year: album_context
                                .as_ref()
                                .and_then(|album| album.year)
                                // Bei einem einzelnen Track nennt nur das Trackobjekt
                                // das Erscheinungsdatum. Ohne diesen Rückgriff bliebe
                                // das Jahr leer und ein altes YouTube-Jahr stehen.
                                .or_else(|| {
                                    object
                                        .get("releaseDate")
                                        .and_then(|date| date.get("isoString"))
                                        .and_then(Value::as_str)
                                        .and_then(parse_year)
                                }),
                            artwork_path: None,
                            artwork_url,
                            duration,
                        });
                        seen.insert(id.to_owned(), ());
                    }
                }
            }
            for child in object.values() {
                collect_tracks(child, out, seen, album_context.clone());
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_tracks(child, out, seen, inherited_album.clone());
            }
        }
        _ => {}
    }
}

fn parse_number(value: &Value) -> Option<u32> {
    value
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .or_else(|| value.as_str().and_then(|number| number.parse().ok()))
}

/// Liest das Jahr aus einem ISO-Datum wie `2025-03-25`.
fn parse_year(value: &str) -> Option<u32> {
    value.get(..4).and_then(|year| year.parse().ok())
}

fn find_artwork(value: &Value) -> Option<&str> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key.to_lowercase().contains("image") || key == "url" || key == "uri" {
                    if let Some(url) = child.as_str() {
                        if url.starts_with("https://")
                            && (url.contains("scdn.co") || url.contains("spotifycdn.com"))
                        {
                            return Some(url);
                        }
                    }
                }
                if let Some(found) = find_artwork(child) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(find_artwork),
        _ => None,
    }
}

fn find_track_count(value: &Value) -> Option<usize> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if matches!(key.as_str(), "totalCount" | "totalTracks" | "total_tracks") {
                    if let Some(count) = child.as_u64() {
                        return Some(count as usize);
                    }
                }
                if let Some(count) = find_track_count(child) {
                    return Some(count);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(find_track_count),
        _ => None,
    }
}

fn page_title(html: &str) -> Option<String> {
    let start = html.find("<title>")? + "<title>".len();
    let end = html[start..].find("</title>")? + start;
    Some(html[start..end].replace("·Spotify", "").trim().to_owned())
}

/// Sucht das Objekt der Auswahl in den eingebetteten Daten.
///
/// Das Objekt trägt den Namen, den Interpreten und die Trackliste.
fn find_entity(value: &Value) -> Option<&Value> {
    match value {
        Value::Object(object) => {
            let is_entity = object
                .get("uri")
                .and_then(Value::as_str)
                .is_some_and(|uri| uri.starts_with("spotify:"));
            if is_entity && object.contains_key("trackList") {
                return Some(value);
            }
            object.values().find_map(find_entity)
        }
        Value::Array(items) => items.iter().find_map(find_entity),
        _ => None,
    }
}

/// Sucht den Namen der Spotify-Auswahl in den eingebetteten Daten.
///
/// Der Seitentitel ist bei Spotify meist leer, der Name steht am Objekt selbst.
fn find_entity_name(value: &Value) -> Option<String> {
    match value {
        Value::Object(object) => {
            let is_entity = object
                .get("uri")
                .and_then(Value::as_str)
                .is_some_and(|uri| uri.starts_with("spotify:"));
            if is_entity {
                if let Some(name) = object.get("name").and_then(Value::as_str) {
                    if !name.trim().is_empty() {
                        return Some(name.trim().to_owned());
                    }
                }
            }
            object.values().find_map(find_entity_name)
        }
        Value::Array(items) => items.iter().find_map(find_entity_name),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn liest_jahr_aus_iso_datum() {
        assert_eq!(parse_year("2025-03-25"), Some(2025));
        assert_eq!(parse_year("2025"), Some(2025));
        assert_eq!(parse_year(""), None);
        assert_eq!(parse_year("abcd-01-01"), None);
    }

    #[test]
    fn erkennt_spotify_links() {
        assert_eq!(
            parse_spotify_url("https://open.spotify.com/track/1bAv1DWUibxCIjAk9SlzRi"),
            Ok((SourceType::Track, "1bAv1DWUibxCIjAk9SlzRi".to_string()))
        );
        assert_eq!(
            parse_spotify_url(
                "https://open.spotify.com/intl-de/album/0fSfkmx0tdPqFYkJuNX74a?si=abc"
            ),
            Ok((SourceType::Album, "0fSfkmx0tdPqFYkJuNX74a".to_string()))
        );
        assert!(parse_spotify_url("https://example.com/track/abc").is_err());
        assert!(parse_spotify_url("https://open.spotify.com/track/abc/extra").is_err());
    }

    #[test]
    fn bevorzugt_exakten_titel_mit_album_und_laufzeit() {
        let value = json!({
            "data": [
                {
                    "id": 1,
                    "title": "2024 (Remix)",
                    "duration": 209,
                    "artist": { "name": "Playboi Carti" },
                    "album": { "title": "2024 (Remix)" }
                },
                {
                    "id": 2,
                    "title": "2024",
                    "duration": 209,
                    "artist": { "name": "Playboi Carti" },
                    "album": { "title": "MUSIC - SORRY 4 DA WAIT" }
                },
                {
                    "id": 3,
                    "title": "2024",
                    "duration": 180,
                    "artist": { "name": "Playboi Carti" },
                    "album": { "title": "Irgendeine Sammlung" }
                }
            ]
        });
        let found = pick_deezer_match(&value, "Playboi Carti", "2024", Some(209)).unwrap();
        assert_eq!(found.track_id, 2);
        assert_eq!(found.album, "MUSIC - SORRY 4 DA WAIT");
    }

    #[test]
    fn verwirft_fremde_interpreten() {
        let value = json!({
            "data": [{
                "id": 1,
                "title": "2024",
                "duration": 209,
                "artist": { "name": "Cover Band" },
                "album": { "title": "Karaoke" }
            }]
        });
        assert!(pick_deezer_match(&value, "Playboi Carti", "2024", Some(209)).is_none());
    }

    #[test]
    fn findet_album_trotz_zusatz_im_namen() {
        let value = json!({
            "data": [{
                "id": 7,
                "title": "MUSIC - SORRY 4 DA WAIT",
                "artist": { "name": "Playboi Carti" }
            }]
        });
        assert_eq!(pick_deezer_album(&value, "MUSIC", "Playboi Carti"), Some(7));
    }

    fn youtube_entry(id: &str, title: &str, channel: &str, duration: u64) -> Value {
        json!({
            "id": id,
            "url": format!("https://www.youtube.com/watch?v={id}"),
            "title": title,
            "channel": channel,
            "duration": duration,
            "channel_is_verified": channel == "OsamaSon",
        })
    }

    #[test]
    fn waehlt_offizielles_video_statt_how_it_was_made() {
        let entries = vec![
            youtube_entry(
                "howmade",
                "(99% ACCURATE) How \"GTFO The Room\" By OsamaSon was made in FL Studio",
                "Coopin",
                282,
            ),
            youtube_entry(
                "extended",
                "Osamason - GTFO The Room (Extended Intro) [prod.rlly]",
                "iflookscouldntkill",
                181,
            ),
            youtube_entry(
                "live",
                "OsamaSon - GTFO The Room Live @ The Novo in Los Angeles",
                "da man",
                105,
            ),
            youtube_entry("official", "OsamaSon - GTFO The Room", "OsamaSon", 141),
            youtube_entry(
                "slowed",
                "OsamaSon ~ GTFO The Room {Slowed + Reverb}",
                "Cha'quavious",
                177,
            ),
        ];
        let found = pick_youtube_candidate(
            &entries,
            "GTFO The Room",
            &["OsamaSon".to_string()],
            Some(141),
        )
        .expect("kein passendes Video gefunden");
        assert_eq!(found.url, "https://www.youtube.com/watch?v=official");
        assert_eq!(found.duration, Some(141));
    }

    #[test]
    fn verwirft_mixe_und_falsche_laufzeiten() {
        let entries = vec![
            youtube_entry(
                "album",
                "OsamaSon - Jump Out V1 (FULL ALBUM)",
                "ricochet",
                5028,
            ),
            youtube_entry(
                "howmade",
                "[99% Accurate] How Mufasa by OsamaSon was made.",
                "suroi",
                738,
            ),
            youtube_entry(
                "slowed",
                "OsamaSon - Mufasa (432hz)",
                "Ethyro",
                108,
            ),
        ];
        assert!(pick_youtube_candidate(
            &entries,
            "Mufasa",
            &["OsamaSon".to_string()],
            Some(107),
        )
        .is_none());
    }

    #[test]
    fn erkennt_lyrics_video_als_rueckfall() {
        let entries = vec![
            youtube_entry(
                "howmade",
                "How \"GTFO The Room\" by OsamaSon was made",
                "9bbb9",
                285,
            ),
            youtube_entry(
                "lyrics",
                "OsamaSon - GTFO The Room (Lyrics)",
                "LyricVault",
                141,
            ),
        ];
        let found = pick_youtube_candidate(
            &entries,
            "GTFO The Room",
            &["OsamaSon".to_string()],
            Some(141),
        )
        .expect("kein Lyrics-Video gefunden");
        assert_eq!(found.title, "OsamaSon - GTFO The Room (Lyrics)");
    }

    #[test]
    fn verwirft_fremden_interpreten() {
        let entries = vec![youtube_entry(
            "fremd",
            "IF THE MUSIC TOO LOUD GTFO THE ROOM (Official Audio)",
            "Baby Monroe",
            115,
        )];
        assert!(pick_youtube_candidate(
            &entries,
            "GTFO The Room",
            &["OsamaSon".to_string()],
            Some(141),
        )
        .is_none());
    }

    #[test]
    fn erkennt_interpret_vor_dem_titel() {
        let entries = vec![youtube_entry(
            "reupload",
            "Mufasa - OsamaSon",
            "babyjump",
            107,
        )];
        let found = pick_youtube_candidate(
            &entries,
            "Mufasa",
            &["OsamaSon".to_string()],
            Some(107),
        )
        .expect("Titel mit Interpret vorne wurde nicht erkannt");
        assert_eq!(found.duration, Some(107));
    }

    #[test]
    fn kurze_titel_werden_nur_exakt_verglichen() {
        let entries = vec![youtube_entry(
            "falsch",
            "OsamaSon - Jumpout",
            "OsamaSon",
            100,
        )];
        assert!(pick_youtube_candidate(
            &entries,
            "Jump",
            &["OsamaSon".to_string()],
            Some(100),
        )
        .is_none());

        let entries = vec![youtube_entry(
            "richtig",
            "OsamaSon - Jump",
            "OsamaSon",
            100,
        )];
        assert!(pick_youtube_candidate(
            &entries,
            "Jump",
            &["OsamaSon".to_string()],
            Some(100),
        )
        .is_some());
    }

    #[test]
    fn suchanfrage_enthaelt_interpret_und_titel() {
        assert_eq!(
            search_query("Mufasa", &["OsamaSon".to_string()]),
            "OsamaSon Mufasa"
        );
    }

    #[test]
    fn laufzeit_toleranz_erkennt_mixe() {
        assert!(duration_is_close(141, 141.0));
        assert!(duration_is_close(107, 115.0));
        assert!(!duration_is_close(141, 281.0));
        assert!(!duration_is_close(107, 738.0));
    }
}
