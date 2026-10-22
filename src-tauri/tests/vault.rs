//! Prüft den Tresor für die Zugangsdaten des Servers.

use silasdownloader_lib::vault::{self, Library};

/// Der Test arbeitet in einem eigenen Ordner, damit der echte Tresor bleibt, wie er ist.
fn eigener_ordner() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("silasdownloader-test-tresor");
    let _ = std::fs::remove_dir_all(&dir);
    std::env::set_var("SILAS_VAULT_DIR", &dir);
    dir
}

#[test]
fn legt_an_oeffnet_und_sperrt_wieder() {
    eigener_ordner();
    let password = "ganz geheim";
    let libraries = vec![Library {
        name: "Shows".to_owned(),
        collection_type: "tvshows".to_owned(),
        path: "/srv/docker/jellyfin/media/Serien".to_owned(),
    }];

    vault::store(
        password,
        "100.100.1.1",
        "mein-server",
        "root",
        "schluessel",
        "http://100.100.1.1:8096",
        &libraries,
    )
    .expect("Der Tresor konnte nicht angelegt werden.");
    assert!(vault::exists(), "Der Tresor fehlt");

    let opened = vault::unlock(password).expect("Der Tresor ließ sich nicht öffnen");
    assert_eq!(opened.host, "100.100.1.1");
    assert_eq!(opened.ssh_alias, "mein-server");
    assert_eq!(opened.ssh_user, "root");
    assert_eq!(opened.api_key, "schluessel");
    assert_eq!(opened.libraries.len(), 1);
    assert_eq!(opened.libraries[0].name, "Shows");
    assert_eq!(opened.libraries[0].collection_type, "tvshows");
    assert_eq!(
        opened.libraries[0].path,
        "/srv/docker/jellyfin/media/Serien"
    );

    // Ein falsches Passwort darf nicht öffnen.
    let error = vault::unlock("falsch").expect_err("Ein falsches Passwort dürfte nicht öffnen");
    assert!(error.contains("Passwort"), "unerwartete Meldung: {error}");

    // Ohne Passwort wird der Tresor gelöscht.
    vault::store("", "", "", "", "", "", &[]).expect("Löschen fehlgeschlagen");
    assert!(!vault::exists(), "Der Tresor wurde nicht gelöscht");
}

#[test]
fn erkennt_tailscale_adressen() {
    assert!(vault::looks_like_tailscale("100.100.1.1"));
    assert!(vault::looks_like_tailscale("100.64.0.1"));
    assert!(!vault::looks_like_tailscale("192.168.1.20"));
    assert!(!vault::looks_like_tailscale("203.0.113.5"));
    assert!(!vault::looks_like_tailscale("keine adresse"));
}
