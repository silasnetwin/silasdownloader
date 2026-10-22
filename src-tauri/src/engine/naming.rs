//! Hilfsfunktionen für Dateinamen.
//!
//! Die Namen, die die Engine anlegt, müssen auf Windows gültig sein und dürfen
//! keine Zeichen enthalten, die der Dateisystem-Ebene oder yt-dlp zu schaden
//! kommen.

/// Macht aus einem Titel einen sicheren Dateinamen.
///
/// Ersetzt die verbotenen Zeichen, entfernt Leerzeichen am Ende und kürzt den
/// Namen auf eine Länge, die Windows und die meisten Dateisysteme verkraften.
pub fn safe_filename(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|character| {
            if matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            ) || character.is_control()
            {
                '_'
            } else {
                character
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches(['.', ' ']).to_owned();
    let result = if trimmed.is_empty() {
        "track".to_owned()
    } else {
        trimmed.chars().take(170).collect()
    };
    result.replace(['{', '}', '%'], "_")
}
