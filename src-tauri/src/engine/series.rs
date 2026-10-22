//! Serien-Seiten (z. B. KiKA) in einzelne Folgen auflösen.
//!
//! Viele Streaming-Seiten bieten eine "Alle Folgen"-Übersicht an, die selbst keine
//! abspielbare Playlist ist. Für KiKA lässt sich die Liste direkt aus der öffentlichen
//! JSON-Schnittstelle lesen. Alles andere wird normal an die Engine übergeben.

use serde_json::Value;

use crate::{
    engine::tools::hidden_command,
    model::{SeriesEpisode, SeriesPreview},
};

/// Holt eine Webseite als Text.
fn fetch_text(url: &str) -> Result<String, String> {
    let out = hidden_command("curl.exe")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg("30")
        .arg("--user-agent")
        .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .arg("--")
        .arg(url)
        .output()
        .map_err(|e| format!("Seite konnte nicht geladen werden: {e}"))?;
    if !out.status.success() {
        return Err("Die Seite konnte nicht geladen werden.".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Holt eine JSON-Schnittstelle.
fn fetch_json(url: &str) -> Result<Value, String> {
    let text = fetch_text(url)?;
    serde_json::from_str(&text).map_err(|e| format!("Antwort war kein gültiges JSON: {e}"))
}

/// Wandelt eine aufgelöste Serie in das einheitliche Listenformat um.
pub fn to_list(series: crate::model::SeriesPreview) -> crate::model::ListPreview {
    crate::model::ListPreview {
        url: series.url,
        title: series.title,
        list_kind: "series".into(),
        seasons: series.seasons,
        entries: series
            .episodes
            .into_iter()
            .map(|episode| crate::model::ListEntry {
                index: episode.index,
                title: episode.title,
                url: episode.url,
                duration: episode.duration,
                season: episode.season,
                episode: episode.episode,
                selected: true,
            })
            .collect(),
    }
}

/// Liest die Staffel aus einer Adresszeile.
pub fn season_from_url(url: &str) -> Option<u32> {
    let query = url.split('?').nth(1)?;
    query.split('&').find_map(|pair| {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?;
        let value = parts.next()?;
        if key.eq_ignore_ascii_case("season") {
            value.trim().parse::<u32>().ok()
        } else {
            None
        }
    })
}

/// Prüft, ob die Adresse auf eine "Alle Folgen"-Seite zeigt.
fn is_series_page(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    if !lower.contains("kika.de") {
        return false;
    }
    lower.contains("/videos/alle-folgen") || lower.contains("/videos/alle-")
}

/// Ermittelt die Kennung der Videoreihe aus der Adresse.
///
/// Beispiel: `https://www.kika.de/mako-einfach-meerjungfrau/videos/alle-folgen-302`
/// ergibt `alle-folgen-302`.
fn subchannel_id(url: &str) -> Option<String> {
    let path = url.split('?').next().unwrap_or(url);
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let position = segments
        .iter()
        .position(|s| s.eq_ignore_ascii_case("videos"))?;
    segments.get(position + 1).map(|s| s.to_string())
}

/// Löst eine Serienseite in ihre Folgen auf.
///
/// Gibt `None` zurück, wenn die Adresse keine Serienseite ist.
pub fn resolve_series(url: &str) -> Result<Option<SeriesPreview>, String> {
    if !is_series_page(url) {
        return Ok(None);
    }
    let season = season_from_url(url);

    let mut episodes: Vec<SeriesEpisode> = Vec::new();
    let mut title = String::new();
    let mut seasons: Vec<u32> = Vec::new();

    // Die Kennung der Videoreihe steckt in der Adresse. Damit laesst sich die
    // oeffentliche Schnittstelle direkt abfragen.
    let mut pages_url = subchannel_id(url).map(|id| {
        format!("https://www.kika.de/ackley/v1/videosubchannels/{id}/videos?page=0&platform=kikade")
    });

    // Rueckfall: die Seite nennt die Kennung der Marke.
    if pages_url.is_none() {
        let html = fetch_text(url)?;
        if let Some(brand) = find_brand(&html) {
            let brand_data = fetch_json(&format!(
                "https://www.kika.de/_next-api/proxy/v1/brands/{brand}"
            ))?;
            pages_url = brand_data
                .get("videoSubchannel")
                .and_then(|v| v.get("videosPageUrl"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            if title.is_empty() {
                title = brand_data
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
            }
        }
    }

    let mut next_page = pages_url;
    if next_page.is_none() {
        return Err("Die Videoreihe konnte nicht gelesen werden.".into());
    }

    let mut guard = 0;
    while let Some(page_url) = next_page {
        guard += 1;
        if guard > 100 {
            break;
        }
        let page = fetch_json(&page_url)?;
        if title.is_empty() {
            if let Some(name) = page
                .get("content")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|item| item.get("broadcastSeriesTitle"))
                .and_then(Value::as_str)
            {
                title = name.to_owned();
            }
        }
        if let Some(items) = page.get("content").and_then(Value::as_array) {
            for item in items {
                let Some(entry) = parse_item(item) else {
                    continue;
                };
                if let Some(number) = entry.season {
                    if !seasons.contains(&number) {
                        seasons.push(number);
                    }
                }
                episodes.push(entry);
            }
        }
        next_page = page
            .get("links")
            .and_then(|links| links.get("next"))
            .and_then(Value::as_str)
            .map(str::to_owned);
    }

    if episodes.is_empty() {
        return Err("In dieser Reihe wurden keine Folgen gefunden.".into());
    }
    seasons.sort_unstable();

    // Die Auswahl nach Staffel einruecken, damit die Nummerierung stimmt.
    let mut selected: Vec<SeriesEpisode> = match season {
        Some(wanted) => episodes
            .iter()
            .filter(|episode| episode.season == Some(wanted))
            .cloned()
            .collect(),
        None => episodes.clone(),
    };
    for (position, episode) in selected.iter_mut().enumerate() {
        episode.index = (position + 1) as u32;
    }

    Ok(Some(SeriesPreview {
        url: url.to_owned(),
        title: if title.is_empty() {
            "Videoreihe".to_owned()
        } else {
            title
        },
        seasons,
        episodes: selected,
    }))
}

/// Liest die Markenkennung aus dem Quelltext einer Seite.
fn find_brand(html: &str) -> Option<String> {
    let marker = "brands/";
    let start = html.find(marker)? + marker.len();
    let rest = &html[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    Some(rest[..end].to_owned())
}

/// Wandelt einen API-Eintrag in eine Folge um.
fn parse_item(item: &Value) -> Option<SeriesEpisode> {
    let api = item.get("api")?;
    let id = api.get("id").and_then(Value::as_str)?;
    let url_path = item
        .get("urlPath")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let url = format!("https://www.kika.de{url_path}/{id}");

    let details = item.get("videoDetails");
    let season = details.and_then(|d| d.get("season")).and_then(|s| match s {
        Value::Number(n) => n.as_u64().map(|v| v as u32),
        Value::String(text) => text.trim().parse::<u32>().ok(),
        _ => None,
    });
    let episode = details
        .and_then(|d| d.get("episodeNumber"))
        .and_then(|s| match s {
            Value::Number(n) => n.as_u64().map(|v| v as u32),
            Value::String(text) => text.trim().parse::<u32>().ok(),
            _ => None,
        });
    let duration = details
        .and_then(|d| d.get("durationInSeconds"))
        .and_then(Value::as_u64);

    let title = item
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or(id)
        .to_owned();

    Some(SeriesEpisode {
        index: 0,
        title,
        url,
        duration,
        season,
        episode,
    })
}
