import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { check } from "@tauri-apps/plugin-updater";
import { confirm } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import {
  ArrowUpCircle,
  Clapperboard,
  Code2,
  Film,
  Music4,
  ShieldCheck,
  Sparkles,
} from "lucide-react";

import { useApp } from "../components/context";
import { useSprache } from "../lib/i18n";

/** Ein Baustein der Übersicht. */
function Feature({
  icon,
  title,
  text,
}: {
  icon: React.ReactNode;
  title: string;
  text: string;
}) {
  return (
    <div className="feature">
      <span className="feature-icon">{icon}</span>
      <div>
        <b>{title}</b>
        <p>{text}</p>
      </div>
    </div>
  );
}

export default function About() {
  const { engine, settings, notify } = useApp();
  const { sprich } = useSprache();
  const [version, setVersion] = useState("…");
  const [checking, setChecking] = useState(false);

  useEffect(() => {
    void getVersion().then(setVersion).catch(() => setVersion("?"));
  }, []);

  const update = async () => {
    setChecking(true);
    try {
      const gefunden = await check();
      if (!gefunden) {
        notify(sprich("Du hast schon die neueste Version."));
        return;
      }
      const text = sprich("Version {version} ist da.", { version: gefunden.version });
      if (await confirm(text, { title: sprich("Update bereit"), kind: "info" })) {
        await gefunden.downloadAndInstall();
        await relaunch();
      }
    } catch (error) {
      notify(String(error), "error");
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="page-inner about">
      <div className="about-hero">
        <img className="about-mark" src="/brand-logo.svg" alt="" />
        <div>
          <div className="eyebrow">
            <span className="eyebrow-line" />
            {sprich("ÜBER")}
          </div>
          <h1>SilasDownloader</h1>
          <p className="about-lead">
            {sprich(
              "Portabler Downloader für Videos, Serien und Musik von beliebigen Seiten. Alles Nötige steckt in der App — kein Python, kein Rust, keine Zusatzprogramme.",
            )}
          </p>
          <div className="about-meta">
            <span className="chip is-active">
              {sprich("Version {version}", { version })}
            </span>
            <span className="chip">yt-dlp {engine.ytdlp}</span>
            <span className="chip">FFmpeg {engine.ffmpeg}</span>
          </div>
        </div>
      </div>

      <div className="card">
        <div className="card-head">
          <div>
            <h2>{sprich("Was die App kann")}</h2>
            <p>{sprich("Drei Bereiche, ein Programm.")}</p>
          </div>
        </div>
        <div className="feature-grid">
          <Feature
            icon={<Clapperboard size={15} />}
            title={sprich("Videos und Serien")}
            text={sprich(
              "Ein Link genügt: die App erkennt selbst, ob es ein Video, eine Playlist oder eine Serie mit allen Folgen ist.",
            )}
          />
          <Feature
            icon={<Music4 size={15} />}
            title={sprich("Musik aus Spotify")}
            text={sprich(
              "Playlist oder Album: jeder Titel wird gesucht, als MP3 geladen und mit Album, Tracknummer, Jahr und quadratischem Cover beschriftet.",
            )}
          />
          <Feature
            icon={<Film size={15} />}
            title={sprich("Konverter")}
            text={sprich(
              "Video in Audio, Tonspur in MP3 oder FLAC, mit Cover und Tags — per Drag & Drop.",
            )}
          />
        </div>
      </div>

      <div className="card">
        <div className="card-head">
          <div>
            <h2>{sprich("Geschwindigkeit")}</h2>
            <p>
              {sprich("YouTube begrenzt die Rate pro Stream. Der Hebel ist die Parallelität.")}
            </p>
          </div>
        </div>
        <div className="meta-grid">
          <div className="meta-cell">
            <span>{sprich("{n} Auftrag", { n: 1 })}</span>
            <b>2,4 MB/s</b>
          </div>
          <div className="meta-cell">
            <span>{sprich("{n} Aufträge", { n: 2 })}</span>
            <b>3,7 MB/s</b>
          </div>
          <div className="meta-cell">
            <span>{sprich("{n} Aufträge", { n: 4 })}</span>
            <b>7,6 MB/s</b>
          </div>
          <div className="meta-cell">
            <span>{sprich("{n} Aufträge", { n: 6 })}</span>
            <b>11,6 MB/s</b>
          </div>
        </div>
        <p style={{ color: "var(--quiet)", fontSize: 10, marginTop: 12 }}>
          {sprich(
            "Gemessen mit denselben sechs Videos. Die Zahl der gleichzeitigen Aufträge steht in den Einstellungen — aktuell {n}.",
            { n: settings.maxParallelDownloads },
          )}
        </p>
      </div>

      <div className="card">
        <div className="card-head">
          <div>
            <h2>{sprich("Technik")}</h2>
            <p>{sprich("Alles steckt in einer Datei.")}</p>
          </div>
        </div>
        <div className="meta-grid">
          <div className="meta-cell">
            <span>{sprich("Oberfläche")}</span>
            <b>React · Tauri 2</b>
          </div>
          <div className="meta-cell">
            <span>{sprich("Backend")}</span>
            <b>Rust</b>
          </div>
          <div className="meta-cell">
            <span>{sprich("JavaScript-Runtime")}</span>
            <b>Deno</b>
          </div>
          <div className="meta-cell">
            <span>{sprich("Turbo-Downloader")}</span>
            <b>aria2c</b>
          </div>
          <div className="meta-cell">
            <span>{sprich("Zielordner")}</span>
            <b>{settings.outputDir}</b>
          </div>
          <div className="meta-cell">
            <span>{sprich("Parallel")}</span>
            <b>{sprich("{n} Aufträge (Anzahl)", { n: settings.maxParallelDownloads })}</b>
          </div>
        </div>
        <div className="job-actions" style={{ marginTop: 16 }}>
          <button className="primary-button" onClick={() => void update()} disabled={checking}>
            <ArrowUpCircle size={13} />{" "}
            {checking ? sprich("Prüfe …") : sprich("Nach Updates suchen")}
          </button>
          <a
            className="ghost-button"
            href="https://github.com/silasnetwin/silasdownloader"
            target="_blank"
            rel="noreferrer"
          >
            <Code2 size={13} /> {sprich("Quellcode")}
          </a>
        </div>
      </div>

      <div className="card">
        <div className="card-head">
          <div>
            <h2>
              <ShieldCheck size={14} /> {sprich("Rechte")}
            </h2>
            <p>{sprich("Bitte beachte die folgenden Hinweise.")}</p>
          </div>
        </div>
        <p style={{ color: "var(--muted)", fontSize: 11, lineHeight: 1.6 }}>
          {sprich(
            "Verwende Downloads nur für Inhalte, für die du die nötigen Rechte oder die Erlaubnis des Rechteinhabers hast. Die App umgeht keine DRM-, Paywall- oder Zugangsbeschränkungen und lädt nur Inhalte, die öffentlich erreichbar sind.",
          )}
        </p>
        <p style={{ color: "var(--quiet)", fontSize: 10, marginTop: 12, lineHeight: 1.6 }}>
          <Sparkles size={11} style={{ verticalAlign: "-1px", marginRight: 4 }} />
          {sprich("Fragen und Verbesserungsvorschläge sind im Repository willkommen.")}
        </p>
      </div>
    </div>
  );
}
