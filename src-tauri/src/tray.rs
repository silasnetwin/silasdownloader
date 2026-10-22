//! Das Symbol im Infobereich (Tray).
//!
//! Es zeigt, wie weit die laufenden Downloads sind, holt das Fenster nach vorn
//! und beendet die App richtig — mit allen Kindprozessen.

use std::sync::{Mutex, OnceLock};

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

/// Die Kennung des Symbols.
pub const TRAY_ID: &str = "main";

/// Zuletzt gezeichneter Stand, damit nicht bei jedem Prozent neu gezeichnet wird.
fn letzter_stand() -> &'static Mutex<(i32, bool)> {
    static STAND: OnceLock<Mutex<(i32, bool)>> = OnceLock::new();
    STAND.get_or_init(|| Mutex::new((-1, false)))
}

/// Legt das Symbol an.
pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let oeffnen = MenuItem::with_id(app, "open", "Fenster zeigen", true, None::<&str>)?;
    let beenden = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&oeffnen, &beenden])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("SilasDownloader")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show(app),
            "quit" => {
                app.state::<crate::AppState>().stop_all();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Holt das Fenster nach vorn.
pub fn show<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Schließt das Fenster nur, wenn der Nutzer das so eingestellt hat.
pub fn close_to_tray() -> bool {
    crate::storage::load_settings().close_to_tray
}

/// Zeigt den Fortschritt im Symbol und im Tooltip.
pub fn update<R: Runtime>(app: &AppHandle<R>, active: u32, total: u32, percent: f32) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let percent = percent.clamp(0.0, 1.0);
    let step = (percent * 100.0).round() as i32;
    let laeuft = active > 0;
    let tooltip = if laeuft {
        format!("SilasDownloader — {active} von {total} · {step} %")
    } else {
        "SilasDownloader — bereit".to_string()
    };
    let _ = tray.set_tooltip(Some(tooltip));

    let neu_zeichnen = match letzter_stand().lock() {
        Ok(mut stand) => {
            let gleich = stand.0 == step && stand.1 == laeuft;
            if !gleich {
                *stand = (step, laeuft);
            }
            !gleich
        }
        Err(_) => true,
    };
    if neu_zeichnen {
        let _ = tray.set_icon(Some(symbol(percent, laeuft)));
    }
}

/// Zeichnet ein 32×32-Symbol mit Fortschrittsring.
fn symbol(percent: f32, active: bool) -> Image<'static> {
    const KANTE: u32 = 32;
    const MITTE: f32 = 16.0;
    const PI: f32 = std::f32::consts::PI;
    let mut bild = vec![0u8; (KANTE * KANTE * 4) as usize];
    for y in 0..KANTE {
        for x in 0..KANTE {
            let dx = x as f32 + 0.5 - MITTE;
            let dy = y as f32 + 0.5 - MITTE;
            let abstand = (dx * dx + dy * dy).sqrt();
            let i = ((y * KANTE + x) * 4) as usize;
            if abstand <= 15.2 {
                bild[i..i + 4].copy_from_slice(&[0x12, 0x17, 0x1c, 0xff]);
            }
            if abstand > 9.6 && abstand <= 13.6 {
                let winkel = dy.atan2(dx) + PI / 2.0;
                let anteil = (winkel / (2.0 * PI)).rem_euclid(1.0);
                let farbe = if active && anteil <= percent {
                    [0x4c, 0xc0, 0x8e, 0xff]
                } else {
                    [0x2c, 0x35, 0x3d, 0xff]
                };
                bild[i..i + 4].copy_from_slice(&farbe);
            }
            if abstand <= 3.4 {
                let farbe = if active {
                    [0x4c, 0xc0, 0x8e, 0xff]
                } else {
                    [0x53, 0x5d, 0x66, 0xff]
                };
                bild[i..i + 4].copy_from_slice(&farbe);
            }
        }
    }
    Image::new_owned(bild, KANTE, KANTE)
}
