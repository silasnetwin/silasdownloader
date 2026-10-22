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
};

use serde_json::Value;

use crate::{
    engine::{naming::safe_filename, tools::hidden_command},
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
/// ewig dauert. Das Cover bekommt den Namen des Titels und liegt damit direkt
/// neben der MP3-Datei.
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

    // Jeder Titel bekommt sein eigenes Bild neben der Datei. Bei einem Album ist
    // das für alle Titel dasselbe Cover, aber so passt das Bild immer zum Titel.
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
            .and_then(|date| date.get(..4))
            .and_then(|year| year.parse::<u32>().ok()),
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

    // Bei Playlists nennt Spotify das Album nicht. Es wird über die Suche von
    // iTunes ermittelt, damit die MP3-Dateien im richtigen Album landen.
    fetch_album_details(&mut tracks);

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
    let artist = artists.join(", ");
    if artist.is_empty() {
        format!("{title} official audio")
    } else {
        format!("{artist} {title} official audio")
    }
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

/// Holt die Albumangaben für Titel, bei denen Spotify sie nicht nennt.
///
/// Die öffentliche Einbettung einer Playlist nennt zu jedem Titel nur Namen und
/// Interpreten. Das Album lässt sich aber über die Suche von iTunes ermitteln.
/// Damit daraus kein falsches Album wird, werden Interpret und Titel genau
/// verglichen und Treffer von fremden Interpreten verworfen.
fn fetch_album_details(tracks: &mut [SpotifyTrack]) {
    // Titel ohne Album brauchen Album, Nummer und Jahr. Titel mit Album, aber ohne
    // Jahr, brauchen nur das Jahr.
    let wanted: Vec<(usize, String, String, bool)> = tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| track.album.trim().is_empty() || track.year.is_none())
        .map(|(index, track)| {
            let artist = track.artists.join(", ");
            let title = strip_title_additions(&track.title);
            (index, artist, title, track.album.trim().is_empty())
        })
        .collect();
    if wanted.is_empty() {
        return;
    }

    for chunk in wanted.chunks(8) {
        let mut children = Vec::new();
        for (index, artist, title, needs_album) in chunk {
            let term = format!("{artist} {title}");
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
                children.push((index, artist, title, *needs_album, child));
            }
        }
        for (index, artist, title, needs_album, child) in children {
            let Ok(output) = child.wait_with_output() else {
                continue;
            };
            let text = String::from_utf8_lossy(&output.stdout);
            let Ok(value) = serde_json::from_str::<Value>(text.trim()) else {
                continue;
            };
            let Some(details) = pick_album_match(&value, artist, title) else {
                continue;
            };
            let track = &mut tracks[*index];
            if needs_album {
                track.album = details.album;
                track.track_number = details.track_number;
                if track.album_artist.trim().is_empty() {
                    track.album_artist = details.album_artist;
                }
            }
            if track.year.is_none() {
                track.year = details.year;
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
                            .and_then(|date| date.get(..4))
                            .and_then(|year| year.parse::<u32>().ok()),
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
                            year: album_context.as_ref().and_then(|album| album.year),
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
