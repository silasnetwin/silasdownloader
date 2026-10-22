//! Ende-zu-Ende-Test der Tauri-Engine.
//!
//! Braucht eine Internetverbindung und wird deshalb nicht automatisch ausgeführt.
//!
//! ```text
//! cargo test --test engine -- --ignored --nocapture
//! ```

use silasdownloader_lib::engine::metadata::analyze_video;

fn sample_url() -> String {
    std::env::var("SMOKE_URL")
        .unwrap_or_else(|_| "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string())
}

#[test]
#[ignore = "benötigt eine Internetverbindung"]
fn analysiert_ein_video_und_holt_das_cover() {
    let preview =
        analyze_video::<tauri::Wry>(None, &sample_url(), "").expect("Analyse fehlgeschlagen");
    assert!(!preview.title.is_empty(), "kein Titel gelesen");
    assert!(!preview.id.is_empty(), "keine ID gelesen");
    assert!(!preview.formats.is_empty(), "keine Formate gelesen");
    assert!(preview.duration.unwrap_or(0) > 0, "keine Laufzeit gelesen");
    let cover = preview
        .thumbnail_data_url
        .expect("kein Vorschaubild geladen");
    assert!(cover.starts_with("data:image/"), "unerwartetes Format");
    // Base64 einer 512er-Kachel ist gut ein paar Kilobyte gross.
    assert!(cover.len() > 2000, "Cover ist unplausibel klein");
    println!("ok: {} · {} Formate", preview.title, preview.formats.len());
}

#[test]
#[ignore = "benötigt eine Internetverbindung"]
fn meldet_fehler_freundlich() {
    let error =
        analyze_video::<tauri::Wry>(None, "https://www.youtube.com/watch?v=notavalidid", "")
            .expect_err("ungültige ID sollte scheitern");
    assert!(!error.is_empty());
    println!("ok: {error}");
}

#[test]
#[ignore = "benötigt eine Internetverbindung"]
fn loest_eine_kika_reihe_in_folgen_auf() {
    let url = "https://www.kika.de/mako-einfach-meerjungfrau/videos/alle-folgen-302?season=3";
    let series = silasdownloader_lib::engine::series::resolve_series(url)
        .expect("Auflösen fehlgeschlagen")
        .expect("Die Seite wurde nicht als Reihe erkannt");

    assert!(
        series.episodes.len() > 5,
        "zu wenige Folgen: {}",
        series.episodes.len()
    );
    assert!(
        series.title.contains("Mako"),
        "unerwarteter Titel: {}",
        series.title
    );
    // Die Staffel 3 wurde ausgewählt.
    assert!(
        series.episodes.iter().all(|e| e.season == Some(3)),
        "nicht alle Folgen gehören zu Staffel 3"
    );
    for episode in &series.episodes {
        assert!(
            episode.url.starts_with("https://www.kika.de/"),
            "unerwartete Adresse: {}",
            episode.url
        );
        assert!(
            episode.duration.unwrap_or(0) > 0,
            "keine Laufzeit: {episode:?}"
        );
    }
    println!(
        "ok: {} · {} Folgen der Staffel 3 (von {} Staffeln)",
        series.title,
        series.episodes.len(),
        series.seasons.len()
    );
    for episode in series.episodes.iter().take(3) {
        println!("   #{} {}", episode.index, episode.title);
    }
}

#[test]
fn erkennt_keine_reihe_bei_normalen_videos() {
    let result =
        silasdownloader_lib::engine::series::resolve_series("https://www.youtube.com/watch?v=x");
    assert!(result.expect("Auflösen fehlgeschlagen").is_none());
}
