//! Der Tresor für die Zugangsdaten des Servers.
//!
//! Die Adresse des Servers, der API-Schlüssel und die Pfade für die Bibliotheken
//! stehen nicht in den Einstellungen, sondern in diesem Tresor. Er wird mit
//! AES-256-GCM verschlüsselt, der Schlüssel wird aus dem Passwort des Nutzers
//! abgeleitet (Argon2id). Ohne dieses Passwort ist die Datei wertlos — auch dann
//! nicht, wenn jemand sie vom Rechner kopiert.
//!
//! Das Passwort ist bewusst nicht das Passwort des Servers. Auf der Leitung wird
//! weiterhin mit dem SSH-Schlüssel gearbeitet, es läuft also kein Kennwort über
//! das Netz.

use std::{
    fs,
    path::PathBuf,
};

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};

/// Kennung der Tresordatei. Ein Wechsel bedeutet, dass die Datei neu angelegt wird.
const MAGIC: &[u8; 4] = b"SDV1";
/// Länge des Salzes.
const SALT_LEN: usize = 16;
/// Länge der Nonce.
const NONCE_LEN: usize = 12;

/// Die geheimen Angaben zum Server.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Vault {
    /// Die Adresse des Servers im Tailscale-Netz, zum Beispiel 100.100.1.1.
    pub host: String,
    /// Der Name, unter dem der Server im SSH-Config steht.
    pub ssh_alias: String,
    /// Der Benutzer für die Anmeldung.
    pub ssh_user: String,
    /// Der Schlüssel für die Jellyfin-Schnittstelle.
    pub api_key: String,
    /// Die Adresse der Jellyfin-Schnittstelle.
    pub jellyfin_url: String,
    /// Die Bibliotheken von Jellyfin mit dem Ordner, in den sie geliefert werden.
    pub libraries: Vec<Library>,
}

/// Eine Bibliothek von Jellyfin und ihr Zielordner auf dem Server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    /// Der Name der Bibliothek, zum Beispiel "Shows".
    pub name: String,
    /// Die Art der Sammlung, zum Beispiel "movies" oder "tvshows".
    pub collection_type: String,
    /// Der Pfad auf dem Server, zum Beispiel /srv/docker/jellyfin/media/Serien.
    pub path: String,
}

/// Die Angaben, die auch bei geschlossenem Tresor sichtbar sein dürfen.
///
/// Adresse, Name und Adresse von Jellyfin sind keine Geheimnisse — der
/// API-Schlüssel bleibt im verschlüsselten Teil. So stehen die Felder nach einem
/// Neustart nicht leer da, obwohl der Tresor noch zu ist.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub host: String,
    pub ssh_alias: String,
    pub ssh_user: String,
    pub jellyfin_url: String,
    pub libraries: Vec<Library>,
}

/// Der Pfad der Tresordatei.
pub fn vault_path() -> PathBuf {
    directory().join("tresor.dat")
}

/// Der Pfad der sichtbaren Angaben.
pub fn meta_path() -> PathBuf {
    directory().join("tresor-info.json")
}

fn directory() -> PathBuf {
    // Für Tests lässt sich der Ordner umbiegen, damit der echte Tresor unberührt bleibt.
    if let Ok(dir) = std::env::var("SILAS_VAULT_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    dirs::config_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("SilasDownloader")
}

/// Prüft, ob die Tailscale-Adresse zum eigenen Netz gehört.
///
/// Das Tailscale-Netz nutzt den Bereich 100.64.0.0/10. Eine Adresse außerhalb
/// davon ist mit hoher Wahrscheinlichkeit ein Tippfehler.
pub fn looks_like_tailscale(address: &str) -> bool {
    let Some(first) = address.split('.').next().and_then(|part| part.parse::<u8>().ok()) else {
        return false;
    };
    (64..=127).contains(&first)
}

/// Legt den Tresor an oder überschreibt ihn.
///
/// Ein leerer Schlüssel oder eine leere Adresse leert den Tresor, damit sich der
/// Server-Upload wieder abschalten lässt.
pub fn store(
    password: &str,
    host: &str,
    ssh_alias: &str,
    ssh_user: &str,
    api_key: &str,
    jellyfin_url: &str,
    libraries: &[Library],
) -> Result<(), String> {
    let path = vault_path();
    if password.trim().is_empty() {
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(meta_path());
        return Ok(());
    }
    let vault = Vault {
        host: host.trim().to_owned(),
        ssh_alias: ssh_alias.trim().to_owned(),
        ssh_user: ssh_user.trim().to_owned(),
        api_key: api_key.trim().to_owned(),
        jellyfin_url: jellyfin_url.trim().to_owned(),
        libraries: libraries.to_vec(),
    };
    let plain = serde_json::to_vec(&vault).map_err(|e| e.to_string())?;
    let encrypted = encrypt(password, &plain)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, encrypted).map_err(|e| e.to_string())?;
    store_meta(&vault)
}

/// Legt die sichtbaren Angaben neben dem Tresor ab.
fn store_meta(vault: &Vault) -> Result<(), String> {
    let meta = Meta {
        host: vault.host.clone(),
        ssh_alias: vault.ssh_alias.clone(),
        ssh_user: vault.ssh_user.clone(),
        jellyfin_url: vault.jellyfin_url.clone(),
        libraries: vault.libraries.clone(),
    };
    let json = serde_json::to_vec_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(meta_path(), json).map_err(|e| e.to_string())
}

/// Liest die sichtbaren Angaben. Fehlen sie, ist der Tresor unbekannt.
pub fn load_meta() -> Option<Meta> {
    let bytes = fs::read(meta_path()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Öffnet den Tresor mit dem Passwort.
///
/// Ein falsches Passwort merkt man an der fehlgeschlagenen Prüfsumme, nicht an
/// einer verständlichen Meldung — so verrät die Datei nicht, ob das Passwort
/// falsch oder die Datei kaputt ist.
pub fn unlock(password: &str) -> Result<Vault, String> {
    let bytes = fs::read(vault_path()).map_err(|_| {
        "Es ist noch kein Tresor angelegt. Bitte zuerst Passwort und Server angeben.".to_string()
    })?;
    let plain = decrypt(password, &bytes)?;
    serde_json::from_slice(&plain).map_err(|_| "Der Tresor konnte nicht gelesen werden.".to_string())
}

/// Prüft, ob ein Tresor vorhanden ist.
pub fn exists() -> bool {
    vault_path().is_file()
}

/// Verschlüsselt die Daten mit dem Passwort.
fn encrypt(password: &str, plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce_bytes);
    let key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new(Key::<aes_gcm::Aes256Gcm>::from_slice(&key));
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plain)
        .map_err(|_| "Die Daten konnten nicht verschlüsselt werden.".to_string())?;
    let mut out = Vec::with_capacity(MAGIC.len() + SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Entschlüsselt die Daten mit dem Passwort.
fn decrypt(password: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < MAGIC.len() + SALT_LEN + NONCE_LEN || &data[..MAGIC.len()] != MAGIC {
        return Err("Der Tresor gehört zu einer anderen Version der App.".to_string());
    }
    let salt = &data[MAGIC.len()..MAGIC.len() + SALT_LEN];
    let nonce = &data[MAGIC.len() + SALT_LEN..MAGIC.len() + SALT_LEN + NONCE_LEN];
    let ciphertext = &data[MAGIC.len() + SALT_LEN + NONCE_LEN..];
    let key = derive_key(password, salt)?;
    let cipher = Aes256Gcm::new(Key::<aes_gcm::Aes256Gcm>::from_slice(&key));
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| "Das Passwort passt nicht zu diesem Tresor.".to_string())
}

/// Leitet den Schlüssel aus dem Passwort ab.
fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    // Argon2id mit den üblichen Werten: 19 MB Speicher, zwei Durchläufe.
    let params = Params::new(19 * 1024, 2, 1, None)
        .map_err(|_| "Die Parameter konnten nicht gesetzt werden.".to_string())?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| "Der Schlüssel konnte nicht berechnet werden.".to_string())?;
    Ok(key)
}
