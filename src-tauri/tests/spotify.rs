//! Prüft das Lesen einer öffentlichen Spotify-Auswahl.

use silasdownloader_lib::spotify::{fetch_public_source, resolve_youtube_match};

#[test]
#[ignore = "benötigt eine Internetverbindung"]
fn liest_titel_und_cover() {
    let temporary = std::env::temp_dir().join("silas-spotify-test");
    let _ = std::fs::remove_dir_all(&temporary);
    let source = fetch_public_source(
        "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M",
        &temporary.join("covers"),
        12,
    )
    .expect("Auswahl konnte nicht gelesen werden");

    println!("Titel: {}", source.title);
    println!("Tracks: {}", source.tracks.len());
    assert!(!source.tracks.is_empty());

    for track in source.tracks.iter().take(3) {
        println!(
            "  {:?} | Künstler {:?} | Album {:?} | Track {:?} | Cover {:?}",
            track.title,
            track.artists,
            track.album,
            track.track_number,
            track.artwork_path
        );
    }

    let with_cover = source
        .tracks
        .iter()
        .filter(|track| track.artwork_path.is_some())
        .count();
    println!("Tracks mit Cover: {with_cover}");
    assert!(
        with_cover > 0,
        "kein Track hat ein Cover - artwork_path bleibt leer"
    );

    // Das Cover muss auch wirklich auf der Platte liegen.
    for track in source.tracks.iter().take(3) {
        if let Some(path) = &track.artwork_path {
            assert!(path.is_file(), "Coverdatei fehlt: {path:?}");
            println!(
                "  Cover: {} ({} Bytes)",
                path.display(),
                std::fs::metadata(path).map(|m| m.len()).unwrap_or_default()
            );
        }
    }

    let _ = std::fs::remove_dir_all(&temporary);
}

#[test]
#[ignore = "benötigt eine Internetverbindung"]
fn findet_das_album_und_die_tracknummer() {
    let temporary = std::env::temp_dir().join("silas-spotify-album-test");
    let _ = std::fs::remove_dir_all(&temporary);

    // Ein Album: Spotify nennt Name, Interpret, Jahr und die Reihenfolge.
    let album = fetch_public_source(
        "https://open.spotify.com/album/1xJHno7SmdVtZAtXbdbDZp",
        &temporary.join("album"),
        12,
    )
    .expect("Album konnte nicht gelesen werden");
    println!("Album: {}", album.title);
    for track in album.tracks.iter().take(4) {
        println!(
            "  {:?} | Album {:?} | Interpret {:?} | Track {:?} | Jahr {:?} | explizit {:?}",
            track.title,
            track.album,
            track.album_artist,
            track.track_number,
            track.year,
            track.explicit
        );
    }
    assert!(!album.tracks.is_empty(), "keine Titel gelesen");
    for (index, track) in album.tracks.iter().enumerate() {
        assert!(!track.album.trim().is_empty(), "Albumname fehlt: {:?}", track.title);
        assert_eq!(
            track.track_number,
            Some(index as u32 + 1),
            "falsche Tracknummer: {:?}",
            track.title
        );
        assert!(track.year.is_some(), "Jahr fehlt: {:?}", track.title);
    }

    // Eine Playlist: Spotify nennt das Album nicht, es wird nachgeschlagen.
    let playlist = fetch_public_source(
        "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M",
        &temporary.join("playlist"),
        12,
    )
    .expect("Playlist konnte nicht gelesen werden");
    let with_album = playlist
        .tracks
        .iter()
        .filter(|track| !track.album.trim().is_empty())
        .count();
    println!("Playlist: {} Titel, davon mit Album: {with_album}", playlist.tracks.len());
    for track in playlist.tracks.iter().take(6) {
        println!(
            "  {:?} | Album {:?} | Track {:?} | Jahr {:?}",
            track.title, track.album, track.track_number, track.year
        );
    }
    assert!(
        with_album * 2 >= playlist.tracks.len(),
        "zu wenige Titel mit Album: {with_album} von {}",
        playlist.tracks.len()
    );

    let _ = std::fs::remove_dir_all(&temporary);
}

#[test]
#[ignore = "benötigt eine Internetverbindung"]
fn findet_richtige_videos_statt_mixe() {
    let bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("bin");
    let yt_dlp = bin.join("yt-dlp.exe");
    let deno = bin.join("deno.exe");
    let tracks = [
        ("GTFO The Room", "OsamaSon", 141u64),
        ("Mufasa", "OsamaSon", 107),
        ("Jumpout", "OsamaSon", 103),
    ];

    for (title, artist, duration) in tracks {
        let found = resolve_youtube_match(
            &yt_dlp,
            &deno,
            title,
            &[artist.to_string()],
            Some(duration),
            false,
        )
        .unwrap_or_else(|error| panic!("{title}: {error}"));
        println!(
            "{title}: {} | {} s | {}",
            found.title,
            found.duration.unwrap_or_default(),
            found.url
        );
        assert!(
            found
                .duration
                .is_some_and(|actual| actual.abs_diff(duration) <= 20),
            "falsche Laufzeit für {title}: {:?} statt {duration}",
            found.duration
        );
    }
}

#[test]
#[ignore = "benötigt eine Internetverbindung"]
fn findet_bei_expliziten_titeln_die_normale_fassung() {
    let bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("bin");
    let yt_dlp = bin.join("yt-dlp.exe");
    let deno = bin.join("deno.exe");
    let tracks = [
        ("Without Me", "Eminem", 290u64),
        ("Not Like Us", "Kendrick Lamar", 274),
    ];

    for (title, artist, duration) in tracks {
        let found = resolve_youtube_match(
            &yt_dlp,
            &deno,
            title,
            &[artist.to_string()],
            Some(duration),
            true,
        )
        .unwrap_or_else(|error| panic!("{title}: {error}"));
        println!(
            "{title}: {} | {} s | {} | {}",
            found.title,
            found.duration.unwrap_or_default(),
            found.channel,
            found.url
        );
        let selected = found.title.to_lowercase();
        for marker in ["clean", "censored", "family friendly", "kidz bop"] {
            assert!(
                !selected.contains(marker),
                "zensierte Fassung für {title}: {}",
                found.title
            );
        }
        assert!(
            found
                .duration
                .is_some_and(|actual| actual.abs_diff(duration) <= 5),
            "falsche Laufzeit für {title}: {:?} statt {duration}",
            found.duration
        );
    }
}

#[test]
#[ignore = "benötigt eine Internetverbindung"]
fn findet_fuer_jedes_album_video_die_richtige_laufzeit() {
    let temporary = std::env::temp_dir().join("silas-spotify-video-test");
    let _ = std::fs::remove_dir_all(&temporary);
    let album = fetch_public_source(
        "https://open.spotify.com/album/5Vb4K4Xf4KPFupmhHi5COM",
        &temporary,
        12,
    )
    .expect("Album konnte nicht gelesen werden");
    assert!(
        album.tracks.iter().any(|track| track.explicit),
        "Spotify-Explizit-Kennzeichnung wurde nicht gelesen"
    );

    let bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("bin");
    let yt_dlp = bin.join("yt-dlp.exe");
    let deno = bin.join("deno.exe");

    for track in &album.tracks {
        let found = resolve_youtube_match(
            &yt_dlp,
            &deno,
            &track.title,
            &track.artists,
            track.duration,
            track.explicit,
        )
        .unwrap_or_else(|error| panic!("{}: {error}", track.title));
        println!(
            "{} | Spotify {:?} | explizit {} | YouTube {:?} | {}",
            track.title, track.duration, track.explicit, found.duration, found.title
        );
        if track.explicit {
            let selected = found.title.to_lowercase();
            let track_title = track.title.to_lowercase();
            assert!(
                ["clean", "censored", "family friendly"].iter().all(|word| {
                    track_title.contains(word) || !selected.contains(word)
                }),
                "zensierte Fassung für expliziten Titel {}: {}",
                track.title,
                found.title
            );
        }
        if let Some(expected) = track.duration {
            // Spotify rundet auf Sekunden. Mehr als drei Sekunden Abweichung
            // bedeuten meist einen anderen Schnitt, etwa mit Musikvideo-Abspann.
            assert!(
                found
                    .duration
                    .is_some_and(|actual| actual.abs_diff(expected) <= 3),
                "falsche Laufzeit für {}: {:?} statt {expected} ({})",
                track.title,
                found.duration,
                found.title
            );
        }
    }

    let _ = std::fs::remove_dir_all(&temporary);
}
