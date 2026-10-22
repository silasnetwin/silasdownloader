//! Startet die Engine mit genau den Argumenten, die auch die App verwendet.
//!
//! So lässt sich herausfinden, welches Argument die Ausgabe unter Windows stört.
//!
//! ```text
//! cargo test --test pipe -- --ignored --nocapture
//! ```

use std::process::{Command, Stdio};

#[test]
#[ignore = "benötigt eine Internetverbindung"]
fn startet_mit_den_arguments_der_app() {
    let tools =
        silasdownloader_lib::engine::tools::resolve_tools::<tauri::Wry>(None).expect("Werkzeuge");
    let work = std::env::temp_dir().join("silas-pipe-probe");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("arbeitsordner");
    let status_file = work.join("status.txt");

    let out_path = work.join("engine-out.log");
    let err_path = work.join("engine-err.log");
    let out_file = std::fs::File::create(&out_path).expect("ausgabedatei");
    let err_file = std::fs::File::create(&err_path).expect("fehlerdatei");

    let mut command = Command::new(&tools.yt_dlp);
    command
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUNBUFFERED", "1")
        .env("PYTHONUTF8", "1")
        .arg("--ignore-config")
        .arg("--js-runtimes")
        .arg(format!("deno:{}", tools.deno.display()))
        .arg("--newline")
        .arg("--progress")
        .arg("--progress-delta")
        .arg("0.2")
        .arg("--windows-filenames")
        .arg("--trim-filenames")
        .arg("180")
        .arg("--retries")
        .arg("10")
        .arg("--fragment-retries")
        .arg("10")
        .arg("--retry-sleep")
        .arg("http:exp=1:20")
        .arg("--retry-sleep")
        .arg("fragment:exp=1:10")
        .arg("--concurrent-fragments")
        .arg("8")
        .arg("--progress-template")
        .arg("download:PROGRESS:%(progress.status)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s|%(info.playlist_index)s")
        .arg("--progress-template")
        .arg("postprocess:POSTPROCESS:%(progress.status)s")
        .arg("--print-to-file")
        .arg("before_dl:TITLE:%(playlist_index)s|%(title)s")
        .arg(&status_file)
        .arg("--print-to-file")
        .arg("after_move:FILE:%(filepath)s")
        .arg(&status_file)
        .arg("--ffmpeg-location")
        .arg(&tools.runtime_dir)
        .arg("--paths")
        .arg(&work)
        .arg("--output")
        .arg("NA - %(title)s [%(id)s].%(ext)s")
        .arg("--throttled-rate")
        .arg("100K")
        .arg("--downloader")
        .arg(format!("http,https:{}", tools.aria2.display()))
        .arg("--downloader")
        .arg("dash,m3u8:native")
        .arg("--downloader-args")
        .arg("aria2c:-x 8 -s 8 -k 1M --file-allocation=none --summary-interval=0 --console-log-level=warn --no-conf=true --max-concurrent-downloads=1 --max-tries=3 --retry-wait=1 --connect-timeout=15")
        .arg("--format")
        .arg("bv*[height<=1080][ext=mp4]+ba[ext=m4a]/b[height<=1080][ext=mp4]/bv*+ba/b[height<=1080]/b")
        .arg("--merge-output-format")
        .arg("mp4")
        .arg("--embed-thumbnail")
        .arg("--embed-metadata")
        .arg("--embed-chapters")
        .arg("--postprocessor-args")
        .arg("Merger:-movflags +faststart")
        .arg("--no-playlist")
        .arg("https://www.kika.de/mako-einfach-meerjungfrau/videos/sechs-neuanfaenge-100")
        .stdout(Stdio::from(out_file))
        .stderr(Stdio::from(err_file))
        .stdin(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command.spawn().expect("Engine starten");
    let status = child.wait().expect("warten");
    let out_lines: Vec<String> = std::fs::read_to_string(&out_path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    let err_lines: Vec<String> = std::fs::read_to_string(&err_path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();

    println!("--- STDOUT: {} Zeilen ---", out_lines.len());
    for line in out_lines.iter().take(12) {
        let bytes: Vec<String> = line
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect();
        let interesting: Vec<String> = bytes
            .iter()
            .enumerate()
            .filter(|(i, _)| *i < 26)
            .map(|(_, b)| b.clone())
            .collect();
        println!("  {line}");
        println!("      bytes: {}", interesting.join(" "));
    }
    println!("--- STDERR: {} Zeilen ---", err_lines.len());
    for line in err_lines.iter().take(25) {
        println!("  {line}");
    }
    println!("--- Status: {status:?} ---");

    let invalid = err_lines
        .iter()
        .any(|line| line.contains("Invalid argument") || line.contains("Errno 22"));
    assert!(!invalid, "die Engine meldet einen Ausgabefehler:\n{err_lines:?}");
    assert!(status.success(), "die Engine endet mit {status:?}");
}
