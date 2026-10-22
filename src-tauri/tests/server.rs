//! Prüft die Verbindung zum Homeserver.
//!
//! Die Angaben kommen aus den Umgebungsvariablen, damit im Repository keine
//! privaten Daten stehen. Ohne diese Angaben werden die Tests übersprungen.
//!
//! ```text
//! $env:SILAS_TEST_HOST = "100.100.1.1"
//! $env:SILAS_TEST_ALIAS = "mein-server"
//! $env:SILAS_TEST_USER = "root"
//! $env:SILAS_TEST_API_KEY = "…"
//! $env:SILAS_TEST_JELLYFIN = "http://100.100.1.1:8096"
//! cargo test --test server -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use silasdownloader_lib::{
    server,
    vault::{self, Library},
};

/// Die Angaben zum Testserver aus der Umgebung.
fn vault() -> Option<vault::Vault> {
    let host = std::env::var("SILAS_TEST_HOST").ok()?;
    let alias = std::env::var("SILAS_TEST_ALIAS").unwrap_or_default();
    let user = std::env::var("SILAS_TEST_USER").unwrap_or_else(|_| "root".to_string());
    let api_key = std::env::var("SILAS_TEST_API_KEY").unwrap_or_default();
    let jellyfin_url = std::env::var("SILAS_TEST_JELLYFIN")
        .unwrap_or_else(|_| format!("http://{host}:8096"));
    Some(vault::Vault {
        host,
        ssh_alias: alias,
        ssh_user: user,
        api_key,
        jellyfin_url,
        libraries: Vec::new(),
    })
}

/// Ohne Angaben gibt es nichts zu prüfen.
macro_rules! need {
    () => {
        match vault() {
            Some(vault) => vault,
            None => {
                eprintln!("SILAS_TEST_HOST fehlt — der Test wird übersprungen.");
                return;
            }
        }
    };
}

#[test]
#[ignore = "benötigt einen Homeserver"]
fn liest_die_bibliotheken_von_jellyfin() {
    let vault = need!();
    let info = server::test(&vault).expect("Jellyfin war nicht erreichbar");
    println!("Jellyfin: {info}");

    let libraries = server::libraries(&vault).expect("Die Bibliotheken kamen nicht");
    for library in &libraries {
        println!("  {} ({}) -> {}", library.name, library.collection_type, library.path);
    }
    // Jede Bibliothek mit Ordner braucht auch eine Art, sonst kann die App sie
    // nicht auseinanderhalten.
    for library in &libraries {
        if !library.path.is_empty() {
            assert!(
                !library.collection_type.is_empty(),
                "die Bibliothek {} hat keine Art",
                library.name
            );
        }
    }
}

#[test]
#[ignore = "benötigt einen Homeserver"]
fn erreicht_den_server_ueber_ssh() {
    let vault = need!();
    server::reachable(&vault).expect("Der Server war nicht erreichbar");
    println!("Der Server antwortet.");
}

#[test]
#[ignore = "benötigt einen Homeserver"]
fn legt_eine_datei_auf_den_server() {
    let vault = need!();
    let alias = vault.ssh_alias.clone();
    let ziel = "/srv/docker/jellyfin/media/Serien/silas-test";
    let temp = std::env::temp_dir().join("silas-server-test.mp4");
    std::fs::write(&temp, b"Testinhalt").expect("Die Testdatei konnte nicht erstellt werden");

    server::upload(&vault, &temp, ziel).expect("Die Datei kam nicht an");

    // Die Datei muss jetzt auf dem Server liegen.
    let output = std::process::Command::new("ssh")
        .args(["-o", "BatchMode=yes", &alias, &format!("ls -la {ziel}/")])
        .output()
        .expect("ssh konnte nicht gestartet werden");
    let text = String::from_utf8_lossy(&output.stdout);
    println!("{text}");
    assert!(
        text.contains("silas-server-test.mp4"),
        "Die Datei fehlt auf dem Server"
    );

    // Aufräumen.
    let _ = std::process::Command::new("ssh")
        .args(["-o", "BatchMode=yes", &alias, &format!("rm -rf {ziel}")])
        .output();
    let _ = std::fs::remove_file(&temp);
}

#[test]
#[ignore = "benötigt einen Homeserver"]
fn findet_die_geraete_im_tailscale_netz() {
    let peers = server::tailscale_peers();
    for peer in &peers {
        println!("  {peer}");
    }
    assert!(!peers.is_empty(), "es wurden keine Geräte gefunden");
}

/// Der Pfad der Tresordatei, damit man sie von Hand löschen kann.
#[test]
fn zeigt_den_pfad_des_tresors() {
    let path: PathBuf = server::vault_file();
    println!("Tresor: {}", path.display());
    assert!(path.to_string_lossy().contains("SilasDownloader"));
}

/// Der Tresor muss die Bibliotheken behalten, wenn er neu geschrieben wird.
#[test]
fn haelt_die_bibliotheken_im_tresor() {
    let libraries = vec![Library {
        name: "Shows".to_owned(),
        collection_type: "tvshows".to_owned(),
        path: "/srv/docker/jellyfin/media/Serien".to_owned(),
    }];
    vault::store(
        "test1234",
        "100.100.1.1",
        "mein-server",
        "root",
        "schluessel",
        "http://100.100.1.1:8096",
        &libraries,
    )
    .expect("Der Tresor konnte nicht angelegt werden");
    let opened = vault::unlock("test1234").expect("Der Tresor ließ sich nicht öffnen");
    assert_eq!(opened.libraries.len(), 1);
    assert_eq!(opened.libraries[0].name, "Shows");
    // Aufräumen: der Tresor soll nach dem Test nicht zurückbleiben.
    vault::store("", "", "", "", "", "", &[]).expect("Löschen fehlgeschlagen");
}
