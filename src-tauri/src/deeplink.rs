//! Adressen der Bauart `silasdownloader://…`.
//!
//! Die App nimmt eine Adresse aus der Befehlszeile (kalter Start), aus einer
//! zweiten Instanz und vom Betriebssystem (Windows-Registrierung) entgegen und
//! reicht sie an die Oberfläche weiter. Dort landet sie im Eingabefeld.

use tauri::{AppHandle, Emitter, Runtime};

/// Das Ereignis, auf das die Oberfläche hört.
pub const EVENT: &str = "app://link";

/// Sucht die erste Adresse in einer Befehlszeile.
pub fn find_in_args<I, S>(args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .map(|arg| arg.as_ref().trim().to_owned())
        .find(|arg| arg.to_ascii_lowercase().starts_with("silasdownloader://"))
}

/// Schickt die Adresse an die Oberfläche.
pub fn emit<R: Runtime>(app: &AppHandle<R>, url: &str) {
    let _ = app.emit(EVENT, url.to_owned());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn findet_die_adresse_zwischen_anderen_argumenten() {
        let args = vec![
            "C:\\Programme\\silasdownloader.exe".to_string(),
            "--flag".to_string(),
            "SilasDownloader://download?url=https%3A%2F%2Fexample.org".to_string(),
        ];
        assert_eq!(
            find_in_args(args).as_deref(),
            Some("SilasDownloader://download?url=https%3A%2F%2Fexample.org")
        );
    }

    #[test]
    fn findet_nichts_ohne_adresse() {
        assert!(find_in_args(["app.exe", "--minimized"]).is_none());
    }
}
