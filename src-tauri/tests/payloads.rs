//! Prüft die Ereignis-Nutzlasten gegen die TypeScript-Definitionen.
//!
//! Das Frontend erwartet bestimmte Feldnamen. Damit ein Tippfehler nicht erst zur
//! Laufzeit auffällt, werden die Payloads hier festgehalten.

use silasdownloader_lib::{
    engine::process::{FilePayload, FinishedPayload, LogPayload, ProgressPayload, TitlePayload},
    model::JobState,
};

/// Liefert die Feldnamen eines JSON-Objekts.
fn keys(json: &serde_json::Value) -> Vec<String> {
    match json.as_object() {
        Some(map) => map.keys().cloned().collect(),
        None => Vec::new(),
    }
}

/// Prüft, dass alle erwarteten Feldnamen vorhanden sind.
fn assert_fields(json: &serde_json::Value, expected: &[&str]) {
    let names = keys(json);
    for field in expected {
        assert!(
            names.iter().any(|name| name == field),
            "Feld {} fehlt in {:?}",
            field,
            names
        );
    }
}

#[test]
fn fortschritt_nutzt_die_feldnamen_des_frontend() {
    let payload = ProgressPayload {
        job_id: 7,
        value: 0.5,
        state: JobState::Downloading,
        status: "Daten werden heruntergeladen".into(),
        speed: "3.2 MB/s".into(),
        eta: "01:20".into(),
        total: "45.0 MB".into(),
    };
    let json = serde_json::to_value(&payload).expect("serialisieren");
    assert_fields(
        &json,
        &["jobId", "value", "state", "status", "speed", "eta", "total"],
    );
    assert_eq!(json["state"], "downloading");
}

#[test]
fn titel_und_datei_nutzen_die_feldnamen_des_frontend() {
    let title = TitlePayload {
        job_id: 1,
        title: "Ein Video".into(),
        playlist_index: Some(3),
    };
    let json = serde_json::to_value(&title).expect("serialisieren");
    assert_fields(&json, &["jobId", "title", "playlistIndex"]);

    let file = FilePayload {
        job_id: 1,
        path: "C:/Video.mp4".into(),
    };
    let json = serde_json::to_value(&file).expect("serialisieren");
    assert_fields(&json, &["jobId", "path"]);
}

#[test]
fn abschluss_nennt_dateien_und_fehler() {
    let payload = FinishedPayload {
        job_id: 4,
        ok: false,
        error: Some("Kaputt".into()),
        technical_details: "Details".into(),
        output_file: None,
        output_files: vec![],
    };
    let json = serde_json::to_value(&payload).expect("serialisieren");
    assert_fields(
        &json,
        &[
            "jobId",
            "ok",
            "error",
            "technicalDetails",
            "outputFile",
            "outputFiles",
        ],
    );
    assert_eq!(json["ok"], false);
}

#[test]
fn protokoll_nennt_die_stufe() {
    let payload = LogPayload {
        job_id: 9,
        level: "error".into(),
        line: "ERROR: etwas".into(),
    };
    let json = serde_json::to_value(&payload).expect("serialisieren");
    assert_fields(&json, &["jobId", "level", "line"]);
    assert_eq!(json["level"], "error");
}
