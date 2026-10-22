//! Die Verbindung zum Server: Jellyfin und der Datei-Transfer.
//!
//! Die Dateien gehen per SCP über das Tailscale-Netz auf den Server. Danach wird
//! der Bibliotheks-Scan angestoßen, damit Jellyfin die neue Datei aufnimmt.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use serde::Serialize;

use crate::{
    engine::tools::hidden_command,
    vault::{Library, Vault},
};

/// Die Angaben zum Server, wie sie in der Oberfläche angezeigt werden.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    /// Ob der Tresor angelegt ist.
    pub configured: bool,
    /// Ob der Tresor in dieser Sitzung geöffnet ist.
    pub unlocked: bool,
    /// Die Adresse des Servers.
    pub host: String,
    /// Der Name, unter dem der Server bekannt ist.
    pub alias: String,
    /// Die Bibliotheken von Jellyfin.
    pub libraries: Vec<Library>,
    /// Der letzte Fehler beim Prüfen der Verbindung.
    pub last_error: Option<String>,
}

/// Liest die Bibliotheken von Jellyfin.
///
/// Der Aufruf nutzt den Header, den auch die Homepage verwendet.
pub fn libraries(vault: &Vault) -> Result<Vec<Library>, String> {
    let url = format!(
        "{}/Library/MediaFolders",
        vault.jellyfin_url.trim_end_matches('/')
    );
    let output = hidden_command("curl.exe")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg("15")
        .arg("--header")
        .arg(format!("Authorization: MediaBrowser Token=\"{}\"", vault.api_key))
        .arg("--")
        .arg(&url)
        .output()
        .map_err(|e| format!("Jellyfin war nicht erreichbar: {e}"))?;
    if !output.status.success() {
        return Err("Jellyfin hat die Anfrage abgelehnt (Schlüssel falsch?).".to_string());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value =
        serde_json::from_str(text.trim()).map_err(|_| "Die Antwort von Jellyfin ist unbekannt.".to_string())?;
    let items = value
        .get("Items")
        .and_then(|items| items.as_array())
        .ok_or("Jellyfin hat keine Bibliotheken gemeldet.")?;
    let mut result = Vec::new();
    for item in items {
        let Some(name) = item.get("Name").and_then(|value| value.as_str()) else {
            continue;
        };
        let art = item
            .get("CollectionType")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned();
        // Der Ordner auf dem Server. Die Bibliothek "Playlists" ist eine virtuelle
        // Sammlung ohne Ordner, für sie gibt es kein Ziel.
        let path = match art.as_str() {
            "tvshows" => "/srv/docker/jellyfin/media/Serien",
            "movies" => "/srv/docker/jellyfin/media/movies",
            _ => "",
        };
        result.push(Library {
            name: name.to_owned(),
            collection_type: art,
            path: path.to_owned(),
        });
    }
    if result.is_empty() {
        return Err("Jellyfin hat keine Bibliotheken gemeldet.".to_string());
    }
    Ok(result)
}

/// Prüft die Verbindung zu Jellyfin.
pub fn test(vault: &Vault) -> Result<String, String> {
    let url = format!(
        "{}/System/Info",
        vault.jellyfin_url.trim_end_matches('/')
    );
    let output = hidden_command("curl.exe")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg("15")
        .arg("--header")
        .arg(format!("Authorization: MediaBrowser Token=\"{}\"", vault.api_key))
        .arg("--")
        .arg(&url)
        .output()
        .map_err(|e| format!("Jellyfin war nicht erreichbar: {e}"))?;
    if !output.status.success() {
        return Err("Jellyfin hat die Anfrage abgelehnt (Schlüssel falsch?).".to_string());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(text.trim()).unwrap_or_default();
    let name = value
        .get("ServerName")
        .and_then(|value| value.as_str())
        .unwrap_or("Jellyfin");
    let version = value
        .get("Version")
        .and_then(|value| value.as_str())
        .unwrap_or("?");
    Ok(format!("{name} {version}"))
}

/// Stößt den Bibliotheks-Scan an.
pub fn trigger_scan(vault: &Vault) -> Result<(), String> {
    let url = format!(
        "{}/Library/Refresh",
        vault.jellyfin_url.trim_end_matches('/')
    );
    let output = hidden_command("curl.exe")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--max-time")
        .arg("20")
        .arg("--request")
        .arg("POST")
        .arg("--header")
        .arg(format!("Authorization: MediaBrowser Token=\"{}\"", vault.api_key))
        .arg("--")
        .arg(&url)
        .output()
        .map_err(|e| format!("Der Scan konnte nicht angestoßen werden: {e}"))?;
    if !output.status.success() {
        return Err("Jellyfin hat den Scan nicht angenommen.".to_string());
    }
    Ok(())
}

/// Kopiert eine Datei auf den Server.
///
/// Die Datei wird über SSH mit `cat` in den Zielordner geschrieben. Das ist
/// robuster als SCP: der Windows-Client kommt mit Leerzeichen im Pfad nicht
/// zurecht, und der Weg über die Shell erlaubt es, den Ordner vorher anzulegen.
pub fn upload(vault: &Vault, local: &Path, remote_dir: &str) -> Result<(), String> {
    if !local.is_file() {
        return Err("Die Datei existiert nicht (mehr).".to_string());
    }
    let name = local
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Der Dateiname lässt sich nicht übertragen.")?;
    let dir = remote_dir.trim_end_matches('/');
    let target = format!("{dir}/{name}");

    // Der Ordner muss existieren, sonst schlägt die Übertragung fehl.
    run_remote(
        vault,
        &format!("mkdir -p {}", quote(dir)),
    )?;

    // Die Datei über die Standardeingabe schicken.
    let file = std::fs::File::open(local).map_err(|e| format!("Die Datei ist nicht lesbar: {e}"))?;
    let mut command = hidden_command("ssh.exe");
    command
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=15")
        .arg(&format!("{}@{}", vault.ssh_user, host_of(vault)))
        .arg(format!("cat > {}", quote(&target)))
        .stdin(std::process::Stdio::from(file))
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let output = command
        .output()
        .map_err(|e| format!("Die Übertragung ist nicht möglich: {e}"))?;
    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr);
        let last = details
            .lines()
            .filter(|line| !line.trim().is_empty())
            .last()
            .unwrap_or("unbekannter Fehler")
            .to_owned();
        return Err(format!("Die Übertragung ist fehlgeschlagen: {last}"));
    }

    // Der Besitzer anpassen, damit Jellyfin die Datei verwalten kann.
    let _ = chown(vault, &target);
    Ok(())
}

/// Setzt den Besitzer der Datei auf den Benutzer des Containers.
fn chown(vault: &Vault, target: &str) -> Result<(), String> {
    run_remote(vault, &format!("chown 1000:1000 -- {}", quote(target)))
}

/// Führt einen Befehl auf dem Server aus.
fn run_remote(vault: &Vault, command: &str) -> Result<(), String> {
    let output = hidden_command("ssh.exe")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=15")
        .arg(&format!("{}@{}", vault.ssh_user, host_of(vault)))
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Der Server war nicht erreichbar: {e}"))?;
    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr);
        let last = details
            .lines()
            .filter(|line| !line.trim().is_empty())
            .last()
            .unwrap_or("unbekannter Fehler")
            .to_owned();
        return Err(last);
    }
    Ok(())
}

/// Setzt einen Pfad in einfache Anführungszeichen.
///
/// Der Befehl läuft auf dem Server in dessen Shell, deshalb müssen Anführungszeichen
/// und Sonderzeichen escaped werden.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Der Name oder die Adresse, unter der der Server erreichbar ist.
///
/// Der Alias aus dem SSH-Config hat Vorrang, damit der Schlüssel und der
/// Host-Schlüssel des Servers greifen.
fn host_of(vault: &Vault) -> String {
    if !vault.ssh_alias.trim().is_empty() {
        return vault.ssh_alias.trim().to_owned();
    }
    vault.host.trim().to_owned()
}

/// Prüft, ob der Server im Tailscale-Netz erreichbar ist.
///
/// Es wird nicht nur die Adresse geprüft, sondern tatsächlich eine Verbindung
/// aufgebaut — so sieht der Nutzer sofort, ob der Server läuft.
pub fn reachable(vault: &Vault) -> Result<(), String> {
    let output = hidden_command("ssh.exe")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg(&format!("{}@{}", vault.ssh_user, host_of(vault)))
        .arg("true")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Der Server war nicht erreichbar: {e}"))?;
    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr);
        let last = details
            .lines()
            .filter(|line| !line.trim().is_empty())
            .last()
            .unwrap_or("unbekannter Fehler")
            .to_owned();
        return Err(format!("Der Server war nicht erreichbar: {last}"));
    }
    Ok(())
}

/// Sucht die Tailscale-Geräte im Netz.
///
/// Damit kann die Oberfläche anbieten, den Server aus einer Liste zu wählen,
/// statt die Adresse abzutippen.
pub fn tailscale_peers() -> Vec<String> {
    let output = hidden_command("tailscale.exe")
        .arg("status")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut peers = Vec::new();
    for line in text.lines().skip(1) {
        let mut parts = line.split_whitespace();
        let Some(address) = parts.next() else {
            continue;
        };
        if !address.starts_with("100.") {
            continue;
        }
        let name = parts.next().unwrap_or_default();
        peers.push(format!("{address}  {name}").trim().to_owned());
    }
    peers
}

/// Der Pfad der Tresordatei, für die Anzeige in der Oberfläche.
pub fn vault_file() -> PathBuf {
    crate::vault::vault_path()
}
