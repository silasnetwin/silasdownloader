import { useEffect, useState } from "react";
import { FolderOpen, Music4, Search, Sparkles } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";

import { useApp } from "../components/context";
import * as api from "../lib/api";
import { useSprache } from "../lib/i18n";
import type { SpotifyPreview, SpotifyProgress } from "../lib/types";
import { formatDuration } from "../lib/format";

interface Track {
  index: number;
  title: string;
  percent: number | null;
  status: string;
  completed: boolean;
  error: string | null;
  outputFile: string | null;
}

const QUALITIES = [320, 256, 192, 128];

export default function Spotify() {
  const { settings, patchSettings, notify } = useApp();
  const { sprich } = useSprache();
  const [url, setUrl] = useState("");
  const [preview, setPreview] = useState<SpotifyPreview | null>(null);
  const [tracks, setTracks] = useState<Track[]>([]);
  const [loading, setLoading] = useState(false);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const unlisten = listen<SpotifyProgress>("spotify://progress", (event) => {
      const update = event.payload;
      setTracks((current) => {
        const next = [...current];
        const position = next.findIndex((track) => track.index === update.index);
        const entry: Track = {
          index: update.index,
          title: update.title,
          percent: update.percent,
          status: update.status,
          completed: update.completed,
          error: update.error,
          outputFile: update.outputFile,
        };
        if (position >= 0) next[position] = entry;
        else next.push(entry);
        return next.sort((a, b) => a.index - b.index);
      });
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, []);

  const analyze = async () => {
    if (!url.trim()) {
      setError(sprich("Bitte einen Spotify-Link einfügen."));
      return;
    }
    setLoading(true);
    setError(null);
    setPreview(null);
    try {
      setPreview(await api.spotifyPreview(url.trim()));
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const chooseFolder = async () => {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string") patchSettings({ outputDir: picked });
  };

  const start = async () => {
    if (!url.trim()) {
      setError(sprich("Bitte einen Spotify-Link einfügen."));
      return;
    }
    setRunning(true);
    setError(null);
    setTracks([]);
    try {
      const result = await api.spotifyWorkflow({
        spotifyUrl: url.trim(),
        outputDir: settings.outputDir,
        audioQuality: settings.audioQuality,
        embedCover: settings.embedThumbnail,
        embedLyrics: settings.embedLyrics,
      });
      notify(
        sprich("{n} MP3 gespeichert", { n: result.downloaded }) +
          (result.failed ? sprich(", {n} fehlgeschlagen", { n: result.failed }) : "") +
          ".",
        result.failed ? "error" : "info",
      );
    } catch (err) {
      setError(String(err));
    } finally {
      setRunning(false);
    }
  };

  const done = tracks.filter((track) => track.completed && !track.error).length;
  const failed = tracks.filter((track) => track.error).length;

  return (
    <div className="page-inner">
      <section className="hero">
        <div className="hero-orbit orbit-one" />
        <div className="hero-orbit orbit-two" />
        <div className="hero-inner">
          <div className="eyebrow">
            <span className="eyebrow-line" />
            {sprich("SPOTIFY")}
          </div>
          <h1>
            {sprich("Eine Playlist, ein")} <span>{sprich("Ordner")}</span>.
          </h1>
          <p>
            {sprich(
              "Spotify-Link einfügen. Jeder Titel wird auf YouTube gesucht, als MP3 geladen und mit Album, Album-Interpret, Tracknummer und Cover beschriftet — ganz ohne Spotify-Konto.",
            )}
          </p>

          <div className="search-panel">
            <div className="search-field">
              <Music4 size={16} />
              <input
                value={url}
                onChange={(event) => setUrl(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && !running) void analyze();
                }}
                placeholder="https://open.spotify.com/playlist/…"
                spellCheck={false}
              />
              <button className="primary-button" onClick={analyze} disabled={loading}>
                <Search size={13} /> {loading ? sprich("Liest …") : sprich("Einlesen")}
              </button>
            </div>
            <div className="search-hints">
              {QUALITIES.map((quality) => (
                <button
                  key={quality}
                  className={`chip${settings.audioQuality === quality ? " is-active" : ""}`}
                  onClick={() => patchSettings({ audioQuality: quality })}
                >
                  {quality} kbit/s
                </button>
              ))}
              <label className="check" style={{ marginLeft: 6 }}>
                <input
                  type="checkbox"
                  checked={settings.embedThumbnail}
                  onChange={(event) => patchSettings({ embedThumbnail: event.target.checked })}
                />
                {sprich("Cover einbetten")}
              </label>
              <label className="check" style={{ marginLeft: 6 }}>
                <input
                  type="checkbox"
                  checked={settings.embedLyrics}
                  onChange={(event) => patchSettings({ embedLyrics: event.target.checked })}
                />
                {sprich("Songtexte einbetten")}
              </label>
            </div>
          </div>
        </div>
      </section>

      {error && (
        <div className="notice is-error" style={{ marginBottom: 14 }}>
          {error}
        </div>
      )}

      {preview && (
        <div className="card" style={{ marginBottom: 14 }}>
          <div className="card-head">
            <div>
              <h2>{preview.title}</h2>
              <p>
                {sprich("{n} Titel gefunden", { n: preview.tracks })}
                {preview.listedTracks > preview.tracks &&
                  ` · ${sprich("{n} laut Spotify", { n: preview.listedTracks })}`}
              </p>
            </div>
            <div className="job-actions" style={{ marginTop: 0 }}>
              <button className="secondary-button" onClick={chooseFolder}>
                <FolderOpen size={13} /> {sprich("Zielordner")}
              </button>
              <button className="primary-button" onClick={start} disabled={running}>
                <Music4 size={13} /> {running ? sprich("Lädt …") : sprich("Als MP3 laden")}
              </button>
            </div>
          </div>
          <div className="list-rows" style={{ maxHeight: 220 }}>
            {preview.items.slice(0, 12).map((item, position) => (
              <div className="list-row" key={`${item.title}-${position}`}>
                <span className="list-index">
                  {String(position + 1).padStart(2, "0")}
                </span>
                {item.cover && (
                  <img className="cover-thumb" src={item.cover} alt="" />
                )}
                <span className="list-name">
                  {item.title}
                  {item.artists && ` — ${item.artists}`}
                  {item.album && <span className="list-tag">{item.album}</span>}
                </span>
                <span className="list-duration">
                  {item.duration !== null ? formatDuration(item.duration) : item.album}
                </span>
              </div>
            ))}
            {preview.items.length > 12 && (
              <div className="list-row" style={{ color: "var(--quiet)" }}>
                <span className="list-index">…</span>
                <span className="list-name">
                  {sprich("und {n} weitere", { n: preview.items.length - 12 })}
                </span>
              </div>
            )}
          </div>
          <p
            style={{
              color: "var(--quiet)",
              fontSize: 10,
              marginTop: 10,
              lineHeight: 1.6,
            }}
          >
            {sprich(
              "Die Cover kommen direkt von Spotify und sind quadratisch. Das Album, die Tracknummer und das Jahr stehen in den Tags — bei einzelnen Tracks und Playlists sucht die App sie nach, weil Spotify sie dort nicht nennt.",
            )}
          </p>
        </div>
      )}

      {tracks.length > 0 && (
        <div className="card">
          <div className="card-head">
            <div>
              <h2>{sprich("Fortschritt")}</h2>
              <p>
                {sprich("{done} fertig", { done })}
                {failed > 0 && ` · ${sprich("{failed} fehlgeschlagen", { failed })}`}
                {` · ${sprich("{n} gestartet", { n: tracks.length })}`}
              </p>
            </div>
            <Sparkles size={15} color="var(--accent2)" />
          </div>
          {tracks.map((track) => (
            <div className="job" key={track.index}>
              <div className="job-head">
                <span
                  className={`job-badge ${
                    track.error
                      ? "is-error"
                      : track.completed
                        ? "is-done"
                        : "is-busy"
                  }`}
                >
                  {track.error
                    ? sprich("Fehlgeschlagen")
                    : track.completed
                      ? sprich("Fertig")
                      : sprich("Lädt")}
                </span>
                <span className="job-title">{track.title}</span>
              </div>
              <div className="progress">
                <i style={{ width: `${Math.round(track.percent ?? 0)}%` }} />
              </div>
              <div className="job-meta">
                <span>{track.error ?? track.status}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
