//! Prüft das Lesen einer öffentlichen Spotify-Auswahl.

use silasdownloader_lib::spotify::fetch_public_source;

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
            "  {:?} | Album {:?} | Interpret {:?} | Track {:?} | Jahr {:?}",
            track.title, track.album, track.album_artist, track.track_number, track.year
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
