import { useEffect, useState } from "react";
import { Trash2 } from "lucide-react";

import { useApp } from "../components/context";
import Section from "../components/Section";
import * as api from "../lib/api";
import { useSprache } from "../lib/i18n";
import type { Language, Preset } from "../lib/types";

export default function SettingsPage() {
  const { settings, patchSettings, engine, logs, clearLogs, notify } = useApp();
  const { sprich } = useSprache();
  const [presets, setPresets] = useState<Preset[]>([]);
  const [presetName, setPresetName] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.listPresets(settings).then(setPresets).catch(() => setPresets([]));
  }, [settings]);

  const number = (value: number, min: number, max: number) =>
    Math.min(Math.max(value, min), max);

  const savePreset = () => {
    const name = presetName.trim();
    if (!name) return;
    const preset: Preset = {
      name,
      mode: settings.mode,
      videoQuality: settings.videoQuality,
      videoContainer: settings.videoContainer,
      audioFormat: settings.audioFormat,
      embedMetadata: settings.embedMetadata,
      embedThumbnail: settings.embedThumbnail,
      subtitles: settings.downloadSubtitles,
    };
    const custom = settings.customPresets.filter((entry) => entry.name !== name);
    patchSettings({ customPresets: [...custom, preset] });
    setPresetName("");
    notify(sprich("Preset „{name}\" gespeichert.", { name }));
  };

  const removePreset = (name: string) => {
    patchSettings({
      customPresets: settings.customPresets.filter((entry) => entry.name !== name),
    });
  };

  return (
    <div className="page-inner">
      <div className="page-head">
        <div className="eyebrow">
          <span className="eyebrow-line" />
          {sprich("EINSTELLUNGEN")}
        </div>
        <h1>{sprich("Einstellungen")}</h1>
        <p>{sprich("Passe Downloader, Verhalten und Engine bis ins Detail an.")}</p>
      </div>

      <Section
        title={sprich("App & Workflow")}
        subtitle={sprich("Design, Startseite und Verhalten.")}
      >
        <div className="grid-3">
          <label className="field">
            <span>{sprich("Design")}</span>
            <select
              className="select"
              value={settings.theme}
              onChange={(event) => patchSettings({ theme: event.target.value })}
            >
              <option value="dark">{sprich("Dunkel")}</option>
              <option value="light">{sprich("Hell")}</option>
            </select>
          </label>
          <label className="field">
            <span>{sprich("Sprache")}</span>
            <select
              className="select"
              value={settings.language}
              onChange={(event) =>
                patchSettings({ language: event.target.value as Language })
              }
            >
              <option value="de">{sprich("Deutsch")}</option>
              <option value="en">{sprich("Englisch")}</option>
            </select>
          </label>
          <label className="field">
            <span>{sprich("Startseite")}</span>
            <select
              className="select"
              value={settings.startPage}
              onChange={(event) => patchSettings({ startPage: event.target.value })}
            >
              <option value="home">{sprich("Home")}</option>
              <option value="downloads">{sprich("Downloads")}</option>
              <option value="spotify">{sprich("Spotify")}</option>
              <option value="converter">{sprich("Konverter")}</option>
              <option value="history">{sprich("Verlauf")}</option>
              <option value="settings">{sprich("Einstellungen")}</option>
            </select>
          </label>
        </div>
        <div className="switch-row" style={{ marginTop: 14 }}>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.duplicateWarning}
              onChange={(event) => patchSettings({ duplicateWarning: event.target.checked })}
            />
            {sprich("Vor doppelten Downloads warnen")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.saveHistory}
              onChange={(event) => patchSettings({ saveHistory: event.target.checked })}
            />
            {sprich("Verlauf speichern")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.openFolderOnComplete}
              onChange={(event) =>
                patchSettings({ openFolderOnComplete: event.target.checked })
              }
            />
            {sprich("Ordner nach Abschluss öffnen")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.closeToTray}
              onChange={(event) => patchSettings({ closeToTray: event.target.checked })}
            />
            {sprich("Schließen legt die App in den Infobereich")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.autostart}
              onChange={(event) => patchSettings({ autostart: event.target.checked })}
            />
            {sprich("Beim Anmelden starten (im Hintergrund)")}
          </label>
        </div>
      </Section>

      <Section
        title={sprich("Eigene Presets")}
        subtitle={sprich("Speichert die aktuellen Downloadoptionen unter einem Namen.")}
      >
        <div className="job-actions" style={{ marginTop: 0 }}>
          <input
            className="input"
            style={{ maxWidth: 260 }}
            placeholder={sprich("Preset-Name")}
            value={presetName}
            onChange={(event) => setPresetName(event.target.value)}
          />
          <button className="primary-button" onClick={savePreset}>
            {sprich("Preset speichern")}
          </button>
        </div>
        {presets.length > 0 && (
          <div className="switch-row" style={{ marginTop: 14 }}>
            {presets.map((preset) => {
              const own = settings.customPresets.some((entry) => entry.name === preset.name);
              return (
                <span
                  key={preset.name}
                  className={`chip${own ? " is-active" : ""}`}
                  style={{ display: "inline-flex", alignItems: "center", gap: 6 }}
                >
                  {preset.name}
                  {own && (
                    <button
                      className="icon-button"
                      style={{ width: 16, height: 16 }}
                      onClick={() => removePreset(preset.name)}
                      title={sprich("Löschen")}
                    >
                      <Trash2 size={11} />
                    </button>
                  )}
                </span>
              );
            })}
          </div>
        )}
      </Section>

      <Section
        title={sprich("Geschwindigkeit & Queue")}
        subtitle={sprich(
          "Parallelität ist der größte Hebel — YouTube begrenzt die Rate pro Stream.",
        )}
      >
        <div className="grid-2">
          <label className="field">
            <span>{sprich("Gleichzeitige Downloads (1-12)")}</span>
            <input
              className="input"
              type="number"
              min={1}
              max={12}
              value={settings.maxParallelDownloads}
              onChange={(event) =>
                patchSettings({
                  maxParallelDownloads: number(Number(event.target.value), 1, 12),
                })
              }
            />
          </label>
          <label className="field">
            <span>{sprich("Verbindungen pro Stream (2–16)")}</span>
            <input
              className="input"
              type="number"
              min={2}
              max={16}
              value={settings.aria2Connections}
              onChange={(event) =>
                patchSettings({ aria2Connections: number(Number(event.target.value), 2, 16) })
              }
            />
          </label>
          <label className="field">
            <span>{sprich("Parallele DASH/HLS-Fragmente (1–32)")}</span>
            <input
              className="input"
              type="number"
              min={1}
              max={32}
              value={settings.concurrentFragments}
              onChange={(event) =>
                patchSettings({ concurrentFragments: number(Number(event.target.value), 1, 32) })
              }
            />
          </label>
          <label className="field">
            <span>{sprich("Retry-Anzahl (0–30)")}</span>
            <input
              className="input"
              type="number"
              min={0}
              max={30}
              value={settings.retries}
              onChange={(event) =>
                patchSettings({ retries: number(Number(event.target.value), 0, 30) })
              }
            />
          </label>
          <label className="field">
            <span>{sprich("Speed-Limit KiB/s (0 = unbegrenzt)")}</span>
            <input
              className="input"
              type="number"
              min={0}
              value={settings.speedLimitKib ?? 0}
              onChange={(event) => {
                const value = Number(event.target.value);
                patchSettings({ speedLimitKib: value === 0 ? null : value });
              }}
            />
          </label>
          <label className="field">
            <span>{sprich("Maximale Dateinamenlänge")}</span>
            <input
              className="input"
              type="number"
              min={40}
              max={240}
              value={settings.filenameMaxLen}
              onChange={(event) =>
                patchSettings({ filenameMaxLen: number(Number(event.target.value), 40, 240) })
              }
            />
          </label>
        </div>

        <div className="switch-row" style={{ marginTop: 16 }}>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.turboDownloads}
              onChange={(event) => patchSettings({ turboDownloads: event.target.checked })}
            />
            {sprich("Turbo-Download (aria2c)")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.faststart}
              onChange={(event) => patchSettings({ faststart: event.target.checked })}
            />
            {sprich("Faststart (Index voran)")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.downloadArchive}
              onChange={(event) => patchSettings({ downloadArchive: event.target.checked })}
            />
            {sprich("Archiv (bereits Geladenes überspringen)")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.playlistSingleRun}
              onChange={(event) => patchSettings({ playlistSingleRun: event.target.checked })}
            />
            {sprich("Playlists in einem Durchgang")}
          </label>
        </div>
      </Section>

      <Section
        title={sprich("Speicherort & Dateinamen")}
        subtitle={sprich("Wo die Dateien landen und wie sie heissen.")}
      >
        <div className="grid-2">
          <label className="field">
            <span>{sprich("Downloadordner")}</span>
            <input
              className="input"
              value={settings.outputDir}
              onChange={(event) => patchSettings({ outputDir: event.target.value })}
            />
          </label>
          <label className="field">
            <span>{sprich("Dateiname-Template")}</span>
            <input
              className="input"
              value={settings.filenameTemplate}
              onChange={(event) => patchSettings({ filenameTemplate: event.target.value })}
            />
          </label>
          <label className="field">
            <span>{sprich("Ordner-Template")}</span>
            <input
              className="input"
              value={settings.videoFolderTemplate}
              onChange={(event) => patchSettings({ videoFolderTemplate: event.target.value })}
            />
          </label>
        </div>
        <p style={{ color: "var(--quiet)", fontSize: 10, marginTop: 10 }}>
          {sprich("Variablen")}: {title("{title}")} {title("{channel}")} {title("{date}")}{" "}
          {title("{id}")} {title("{playlist_index}")}
        </p>
        <div className="switch-row" style={{ marginTop: 10 }}>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.separateFolderPerVideo}
              onChange={(event) =>
                patchSettings({ separateFolderPerVideo: event.target.checked })
              }
            />
            {sprich("Für jedes Video einen eigenen Unterordner")}
          </label>
        </div>
      </Section>

      <Section
        title={sprich("Metadaten & Zusatzdateien")}
        subtitle={sprich("Steuere, wie viele Metadaten zu einer Datei dazukommen.")}
      >
        <div className="switch-row">
          <label className="check">
            <input
              type="checkbox"
              checked={settings.embedMetadata}
              onChange={(event) => patchSettings({ embedMetadata: event.target.checked })}
            />
            {sprich("Metadaten einbetten")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.embedThumbnail}
              onChange={(event) => patchSettings({ embedThumbnail: event.target.checked })}
            />
            {sprich("Cover einbetten")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.embedChapters}
              onChange={(event) => patchSettings({ embedChapters: event.target.checked })}
            />
            {sprich("Kapitel einbetten")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.writeInfoJson}
              onChange={(event) => patchSettings({ writeInfoJson: event.target.checked })}
            />
            {sprich(".info.json schreiben")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.writeDescription}
              onChange={(event) => patchSettings({ writeDescription: event.target.checked })}
            />
            {sprich(".description schreiben")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.downloadSubtitles}
              onChange={(event) => patchSettings({ downloadSubtitles: event.target.checked })}
            />
            {sprich("Untertitel laden")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={settings.autoSubtitles}
              onChange={(event) => patchSettings({ autoSubtitles: event.target.checked })}
            />
            {sprich("Automatische Untertitel")}
          </label>
        </div>
        <div className="grid-2" style={{ marginTop: 14 }}>
          <label className="field">
            <span>{sprich("Untertitel-Sprachen")}</span>
            <input
              className="input"
              value={settings.subtitleLanguages}
              onChange={(event) => patchSettings({ subtitleLanguages: event.target.value })}
            />
          </label>
          <label className="field">
            <span>{sprich("Untertitel-Format")}</span>
            <select
              className="select"
              value={settings.subtitleFormat}
              onChange={(event) => patchSettings({ subtitleFormat: event.target.value })}
            >
              <option value="srt">SRT</option>
              <option value="vtt">VTT</option>
            </select>
          </label>
        </div>
      </Section>

      <Section
        title={sprich("Engine-Optionen (für Profis)")}
        subtitle={sprich("Werte werden direkt an yt-dlp weitergegeben.")}
        defaultOpen={false}
      >
        <div className="grid-2">
          <label className="field">
            <span>{sprich("Extractor-Argumente")}</span>
            <input
              className="input"
              placeholder="youtube:player_client=default,-tv"
              value={settings.extractorArgs}
              onChange={(event) => patchSettings({ extractorArgs: event.target.value })}
            />
          </label>
          <label className="field">
            <span>{sprich("Zusatz-Argumente")}</span>
            <input
              className="input"
              placeholder="--format-sort res,fps,size"
              value={settings.extraYtdlpArgs}
              onChange={(event) => patchSettings({ extraYtdlpArgs: event.target.value })}
            />
          </label>
        </div>
      </Section>

      <Section
        title={sprich("Engine & Updates")}
        subtitle={sprich("Die mitgelieferten Werkzeuge und ihre Versionen.")}
        defaultOpen={false}
      >
        <div className="meta-grid">
          <div className="meta-cell">
            <span>yt-dlp</span>
            <b>{engine.ytdlp}</b>
          </div>
          <div className="meta-cell">
            <span>FFmpeg</span>
            <b>{engine.ffmpeg}</b>
          </div>
          <div className="meta-cell">
            <span>{sprich("Oberfläche")}</span>
            <b>React · Tauri 2</b>
          </div>
        </div>
        <div className="job-actions" style={{ marginTop: 14 }}>
          <button
            className="ghost-button"
            onClick={() => {
              setBusy(true);
              void api
                .updateEngine()
                .then((version) =>
                  notify(sprich("yt-dlp ist jetzt auf Version {version}.", { version })),
                )
                .catch((error) => notify(String(error), "error"))
                .finally(() => setBusy(false));
            }}
            disabled={busy}
          >
            {sprich("yt-dlp aktualisieren")}
          </button>
        </div>
        <p style={{ color: "var(--quiet)", fontSize: 10, marginTop: 12 }}>
          {sprich(
            "yt-dlp geht alle paar Wochen kaputt, wenn sich eine Seite ändert. Die Aktualisierung holt die neueste Version von GitHub. Vorher bitte laufende Downloads beenden.",
          )}
        </p>
      </Section>

      <Section
        title={sprich("Protokoll")}
        subtitle={sprich("Die Ausgabe der Download-Engine.")}
        defaultOpen={false}
        action={
          <button className="ghost-button" onClick={clearLogs}>
            {sprich("Leeren")}
          </button>
        }
      >
        <div className="log">
          {logs.length === 0 ? sprich("Noch keine Einträge.") : logs.join("\n")}
        </div>
      </Section>
    </div>
  );
}

function title(value: string) {
  return (
    <code
      style={{
        background: "var(--elev)",
        color: "var(--accent2)",
        borderRadius: 3,
        padding: "2px 5px",
        marginRight: 4,
      }}
    >
      {value}
    </code>
  );
}
