//! Songtexte von LRCLIB holen.
//!
//! LRCLIB ist eine freie Sammlung ohne Schlüssel und ohne Anmeldung. Der Abruf
//! läuft über das Windows-eigene `curl.exe`, so wie die übrigen Netzabrufe der
//! App auch. Findet sich kein Text, bleibt der Rahmen einfach weg.

use crate::engine::tools::hidden_command;

/// Kennung gegenüber dem Dienst (LRCLIB bittet darum).
const USER_AGENT: &str = "SilasDownloader/1.3 (https://github.com/silasnetwin/silasdownloader)";

/// Holt den Songtext zu einem Titel.
///
/// Zuerst wird der genaue Treffer abgefragt, danach die Suche. Zurück kommt der
/// einfache Text ohne Zeitmarken, weil der sich überall anzeigen lässt.
pub fn fetch(artist: &str, title: &str, album: &str, duration: Option<u64>) -> Option<String> {
    let artist = artist.trim();
    let title = title.trim();
    if artist.is_empty() || title.is_empty() {
        return None;
    }

    let mut url = format!(
        "https://lrclib.net/api/get?artist_name={}&track_name={}",
        encode(artist),
        encode(title)
    );
    if !album.trim().is_empty() {
        url.push_str(&format!("&album_name={}", encode(album.trim())));
    }
    if let Some(seconds) = duration.filter(|seconds| *seconds > 0) {
        url.push_str(&format!("&duration={seconds}"));
    }
    if let Some(text) = request(&url).and_then(|json| text_of(&json)) {
        return Some(text);
    }

    let suche = format!(
        "https://lrclib.net/api/search?artist_name={}&track_name={}",
        encode(artist),
        encode(title)
    );
    let json = request(&suche)?;
    json.as_array()?
        .iter()
        .find_map(|treffer| text_of(treffer))
}

/// Holt eine Adresse und liest die Antwort als JSON.
fn request(url: &str) -> Option<serde_json::Value> {
    let output = hidden_command("curl.exe")
        .args([
            "-sS",
            "-L",
            "--max-time",
            "12",
            "-H",
            &format!("User-Agent: {USER_AGENT}"),
            "-H",
            "Accept: application/json",
            "-w",
            "\n%{http_code}",
            url,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let roh = String::from_utf8_lossy(&output.stdout);
    let (rumpf, status) = roh.rsplit_once('\n')?;
    if status.trim() != "200" {
        return None;
    }
    serde_json::from_str(rumpf).ok()
}

/// Liest den Text aus einem Treffer.
fn text_of(treffer: &serde_json::Value) -> Option<String> {
    if treffer.get("instrumental").and_then(|wert| wert.as_bool()) == Some(true) {
        return None;
    }
    let text = treffer
        .get("plainLyrics")
        .and_then(|wert| wert.as_str())
        .map(str::to_owned)
        .or_else(|| {
            treffer
                .get("syncedLyrics")
                .and_then(|wert| wert.as_str())
                .map(ohne_zeitmarken)
        })?;
    let text = text.trim().to_owned();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// Entfernt Zeitmarken wie `[00:12.34]` aus einem synchronisierten Text.
fn ohne_zeitmarken(text: &str) -> String {
    text.lines()
        .map(|zeile| match zeile.find(']') {
            Some(ende) if zeile.starts_with('[') => zeile[ende + 1..].trim_start().to_owned(),
            _ => zeile.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Kodiert einen Wert für die Adresszeile.
fn encode(wert: &str) -> String {
    let mut ausgabe = String::with_capacity(wert.len() * 2);
    for byte in wert.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                ausgabe.push(*byte as char)
            }
            _ => ausgabe.push_str(&format!("%{byte:02X}")),
        }
    }
    ausgabe
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kodiert_leerzeichen_und_umlaute() {
        assert_eq!(encode("Böhse Onkelz"), "B%C3%B6hse%20Onkelz");
    }

    #[test]
    fn entfernt_zeitmarken() {
        assert_eq!(ohne_zeitmarken("[00:12.34] Zeile\n[00:15.00]Noch eine"), "Zeile\nNoch eine");
    }

    #[test]
    fn liest_nur_echte_texte() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"instrumental":true,"plainLyrics":null}"#).unwrap();
        assert!(text_of(&json).is_none());
        let json: serde_json::Value =
            serde_json::from_str(r#"{"plainLyrics":"  Zeile  "}"#).unwrap();
        assert_eq!(text_of(&json).as_deref(), Some("Zeile"));
    }
}
