import { useEffect, useState } from "react";
import {
  ChevronDown,
  Download,
  FolderOpen,
  Layers,
  Link2,
  ListChecks,
  ListX,
  Loader2,
  Play,
  Search,
  Sparkles,
} from "lucide-react";

import { useApp } from "../components/context";
import * as api from "../lib/api";
import { useSprache } from "../lib/i18n";
import type {
  DownloadRequest,
  LinkResult,
  ListPreview,
  Preset,
  VideoPreview,
} from "../lib/types";
import { formatDuration, formatUploadDate } from "../lib/format";

const QUALITIES: Array<{ value: NonNullable<Settings["videoQuality"]>; label: string }> = [
  { value: "best", label: "Beste Qualität" },
  { value: "p2160", label: "4K / 2160p" },
  { value: "p1440", label: "1440p" },
  { value: "p1080", label: "1080p" },
  { value: "p720", label: "720p" },
  { value: "p480", label: "480p" },
  { value: "smallest", label: "Kleinste Datei" },
];

const CONTAINERS: Array<{ value: NonNullable<Settings["videoContainer"]>; label: string }> = [
  { value: "mp4", label: "MP4" },
  { value: "mkv", label: "MKV" },
  { value: "webm", label: "WebM" },
];

const AUDIO: Array<{ value: NonNullable<Settings["audioFormat"]>; label: string }> = [
  { value: "mp3", label: "MP3" },
  { value: "m4a", label: "M4A" },
  { value: "opus", label: "Opus" },
  { value: "flac", label: "FLAC" },
  { value: "wav", label: "WAV" },
];

type Settings = import("../lib/types").Settings;

export default function Home() {
  const { settings, patchSettings, enqueue, notify, setPage } = useApp();
  const { sprich } = useSprache();
  const [input, setInput] = useState("");
  const [result, setResult] = useState<LinkResult | null>(null);
  const [list, setList] = useState<ListPreview | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showOptions, setShowOptions] = useState(false);
  const [presets, setPresets] = useState<Preset[]>([]);

  useEffect(() => {
    api.listPresets(settings).then(setPresets).catch(() => setPresets([]));
  }, [settings]);

  // Ein Link, der aus dem Browser ins Fenster gezogen wurde.
  useEffect(() => {
    const handler = (event: Event) => {
      const url = (event as CustomEvent<string>).detail;
      if (!url) return;
      void analyze(url);
    };
    window.addEventListener("silas:link", handler);
    return () => window.removeEventListener("silas:link", handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings.extractorArgs]);

  const analyze = async (ziel?: string) => {
    const target = (ziel ?? input).trim();
    if (!target) {
      setError(sprich("Bitte einen Link einfügen."));
      return;
    }
    setInput(target);
    setLoading(true);
    setError(null);
    setResult(null);
    setList(null);
    try {
      const found = await api.analyzeLink(target, settings.extractorArgs);
      if (found.kind === "list") {
        setList(found);
      } else {
        setResult(found);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const moreFromPage = async () => {
    const target = input.trim();
    if (!target) return;
    setLoading(true);
    setError(null);
    try {
      const found = await api.listPage(target, settings.extractorArgs);
      setList(found);
      setResult(null);
    } catch (err) {
      notify(String(err), "error");
    } finally {
      setLoading(false);
    }
  };

  const startVideo = (video: VideoPreview) => {
    const request: DownloadRequest = {
      url: video.url,
      settings,
      playlistIndex: null,
      playlistBatch: null,
      selectedVideoFormat: null,
      selectedAudioFormat: null,
      // Ein einzelnes Video ist ein Film.
      kind: "movie",
    };
    enqueue({ url: video.url, title: video.title, request });
    notify(sprich("„{title}\" ist in der Queue.", { title: video.title }));
    setPage("downloads");
  };

  const startList = () => {
    if (!list) return;
    const chosen = list.entries.filter((entry) => entry.selected);
    if (chosen.length === 0) return;
    const folder = `${settings.outputDir}\\${list.title.replace(/[<>:"/\\|?*]/g, "_")}`;
    for (const entry of chosen) {
      const request: DownloadRequest = {
        url: entry.url,
        settings: {
          ...settings,
          outputDir: folder,
          filenameTemplate: settings.filenameTemplate.includes("{playlist_index}")
            ? settings.filenameTemplate
            : `{playlist_index} - ${settings.filenameTemplate}`,
        },
        playlistIndex: entry.index,
        playlistBatch: null,
        selectedVideoFormat: null,
        selectedAudioFormat: null,
        // Eine Liste mit Staffeln ist eine Serie, eine einfache Playlist sind
        // meistens Filme. So landet jede Datei in der passenden Bibliothek.
        kind: list.listKind === "series" || list.entries.some((e) => e.season !== null)
          ? "series"
          : "movie",
      };
      enqueue({
        url: entry.url,
        title: entry.title,
        request,
        playlistIndex: entry.index,
      });
    }
    notify(sprich("{count} Videos sind in der Queue.", { count: chosen.length }));
    setPage("downloads");
  };

  const toggleAll = (value: boolean) => {
    setList((current) =>
      current
        ? {
            ...current,
            entries: current.entries.map((entry) => ({ ...entry, selected: value })),
          }
        : current,
    );
  };

  const toggle = (index: number) => {
    setList((current) =>
      current
        ? {
            ...current,
            entries: current.entries.map((entry) =>
              entry.index === index ? { ...entry, selected: !entry.selected } : entry,
            ),
          }
        : current,
    );
  };

  const applyPreset = (preset: Preset) => {
    patchSettings({
      mode: preset.mode,
      videoQuality: preset.videoQuality,
      videoContainer: preset.videoContainer,
      audioFormat: preset.audioFormat,
      embedMetadata: preset.embedMetadata,
      embedThumbnail: preset.embedThumbnail,
      downloadSubtitles: preset.subtitles,
    });
    notify(sprich("„{name}\" übernommen.", { name: preset.name }));
  };

  const video = result && result.kind === "video" ? result : null;
  const isVideo = settings.mode === "video";
  const chosen = list?.entries.filter((entry) => entry.selected).length ?? 0;

  return (
    <div className="page-inner">
      <section className="hero">
        <div className="hero-orbit orbit-one" />
        <div className="hero-orbit orbit-two" />
        <div className="hero-inner">
          <div className="eyebrow">
            <span className="eyebrow-line" />
            {sprich("SILAS DOWNLOADER")}
          </div>
          <h1>
            {sprich("Alles von")} <span>{sprich("überall")}</span>.
          </h1>
          <p>
            {sprich(
              "Link einfügen — die App findet das Video, die Serie oder die Playlist und lädt sie im Hintergrund.",
            )}
          </p>

          <div className="search-panel">
            <div className="search-field">
              <Link2 size={16} />
              <input
                value={input}
                onChange={(event) => setInput(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") void analyze();
                }}
                placeholder={sprich("Video-, Playlist- oder Serienlink einfügen")}
                spellCheck={false}
              />
              <button
                className="primary-button"
                onClick={() => void analyze()}
                disabled={loading || input.trim().length === 0}
              >
                {loading ? (
                  <>
                    <Loader2 size={13} className="spin" /> {sprich("Suche läuft")}
                  </>
                ) : (
                  <>
                    <Search size={13} /> {sprich("Holen")}
                  </>
                )}
              </button>
            </div>
            <div className="search-hints">
              {presets.map((preset) => (
                <button
                  key={preset.name}
                  className="chip"
                  onClick={() => applyPreset(preset)}
                  title={`${preset.mode === "audio" ? sprich("Nur Audio") : preset.videoQuality} · ${preset.videoContainer}`}
                >
                  {preset.name}
                </button>
              ))}
              <span className="search-hint">
                {settings.outputDir}
              </span>
            </div>
          </div>
        </div>
      </section>

      {error && (
        <div className="notice is-error" style={{ marginBottom: 14 }}>
          {error}
        </div>
      )}

      {/* ---------- Ergebnis: ein einzelnes Video ---------- */}
      {video && (
        <div className="card result-card">
          <div className="result-media">
            {video.thumbnailDataUrl && (
              <img className="preview-thumb" src={video.thumbnailDataUrl} alt={video.title} />
            )}
          </div>
          <div className="result-body">
            <h2>{video.title}</h2>
            <div className="result-meta">
              {video.channel && <span>{video.channel}</span>}
              <span>{formatDuration(video.duration)}</span>
              <span>{formatUploadDate(video.uploadDate)}</span>
              <span>
                {sprich("{n} Streams", { n: video.formats.length })}
              </span>
            </div>
            <div className="result-actions">
              <button className="primary-button" onClick={() => startVideo(video)}>
                <Download size={13} /> {sprich("Herunterladen")}
              </button>
              <button className="secondary-button" onClick={() => void moreFromPage()}>
                <Layers size={13} /> {sprich("Weitere Videos auf dieser Seite")}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ---------- Ergebnis: eine Liste von Videos ---------- */}
      {list && (
        <div className="card result-card">
          <div className="card-head">
            <div>
              <h2>{list.title}</h2>
              <p>
                {sprich("{n} Videos auf der Seite gefunden", { n: list.entries.length })}
                {list.seasons.length > 0 &&
                  ` · ${sprich("{n} Staffeln", { n: list.seasons.length })}`}
              </p>
            </div>
            <div className="job-actions" style={{ marginTop: 0 }}>
              <button className="ghost-button" onClick={() => toggleAll(true)}>
                <ListChecks size={12} /> {sprich("Alle")}
              </button>
              <button className="ghost-button" onClick={() => toggleAll(false)}>
                <ListX size={12} /> {sprich("Keine")}
              </button>
              <button className="primary-button" onClick={startList} disabled={chosen === 0}>
                <Play size={12} /> {sprich("{n} laden", { n: chosen })}
              </button>
            </div>
          </div>

          <div className="list-rows">
            {list.entries.map((entry) => (
              <label className="list-row" key={entry.url}>
                <input
                  type="checkbox"
                  checked={entry.selected}
                  onChange={() => toggle(entry.index)}
                />
                <span className="list-index">
                  {String(entry.index).padStart(2, "0")}
                </span>
                <span className="list-name">{entry.title}</span>
                {entry.season !== null && (
                  <span className="list-tag">
                    {sprich("Staffel {n}", { n: entry.season })}
                  </span>
                )}
                <span className="list-duration">{formatDuration(entry.duration)}</span>
              </label>
            ))}
          </div>
        </div>
      )}

      {/* ---------- Optionen: erst sichtbar, wenn man sie braucht ---------- */}
      <div className="card options-card">
        <button className="options-toggle" onClick={() => setShowOptions((v) => !v)}>
          <div className="options-summary">
            <span className="options-title">{sprich("Einstellungen für diesen Download")}</span>
            <span className="options-hint">
              {isVideo
                ? `${sprich(QUALITIES.find((q) => q.value === settings.videoQuality)?.label ?? "")} · ${
                    CONTAINERS.find((c) => c.value === settings.videoContainer)?.label
                  }`
                : AUDIO.find((a) => a.value === settings.audioFormat)?.label}
              {" · "}
              {sprich("{n} parallel", { n: settings.maxParallelDownloads })}
            </span>
          </div>
          <ChevronDown size={16} className={showOptions ? "chevron is-open" : "chevron"} />
        </button>

        {showOptions && (
          <div className="options-body">
            <div className="switch-row" style={{ marginBottom: 14 }}>
              <button
                className={`chip${isVideo ? " is-active" : ""}`}
                onClick={() => patchSettings({ mode: "video" })}
              >
                {sprich("Video")}
              </button>
              <button
                className={`chip${!isVideo ? " is-active" : ""}`}
                onClick={() => patchSettings({ mode: "audio" })}
              >
                {sprich("Nur Audio")}
              </button>
            </div>

            <div className="grid-3">
              {isVideo ? (
                <>
                  <label className="field">
                    <span>{sprich("Qualität")}</span>
                    <select
                      className="select"
                      value={settings.videoQuality}
                      onChange={(event) =>
                        patchSettings({
                          videoQuality: event.target.value as Settings["videoQuality"],
                        })
                      }
                    >
                      {QUALITIES.map((q) => (
                        <option key={q.value} value={q.value}>
                          {sprich(q.label)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="field">
                    <span>{sprich("Container")}</span>
                    <select
                      className="select"
                      value={settings.videoContainer}
                      onChange={(event) =>
                        patchSettings({
                          videoContainer: event.target.value as Settings["videoContainer"],
                        })
                      }
                    >
                      {CONTAINERS.map((c) => (
                        <option key={c.value} value={c.value}>
                          {c.label}
                        </option>
                      ))}
                    </select>
                  </label>
                </>
              ) : (
                <label className="field">
                  <span>{sprich("Audioformat")}</span>
                  <select
                    className="select"
                    value={settings.audioFormat}
                    onChange={(event) =>
                      patchSettings({
                        audioFormat: event.target.value as Settings["audioFormat"],
                      })
                    }
                  >
                    {AUDIO.map((a) => (
                      <option key={a.value} value={a.value}>
                        {a.label}
                      </option>
                    ))}
                  </select>
                </label>
              )}

              <label className="field">
                <span>{sprich("Zielordner")}</span>
                <div className="inline-field">
                  <input
                    className="input"
                    value={settings.outputDir}
                    onChange={(event) => patchSettings({ outputDir: event.target.value })}
                  />
                  <button
                    className="secondary-button"
                    title={sprich("Ordner öffnen")}
                    onClick={() =>
                      void api.revealPath(settings.outputDir).catch((err) => notify(String(err), "error"))
                    }
                  >
                    <FolderOpen size={13} />
                  </button>
                </div>
              </label>
            </div>

            <div className="switch-row" style={{ marginTop: 14 }}>
              <label className="check">
                <input
                  type="checkbox"
                  checked={settings.embedMetadata}
                  onChange={(event) => patchSettings({ embedMetadata: event.target.checked })}
                />
                {sprich("Metadaten")}
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={settings.embedThumbnail}
                  onChange={(event) => patchSettings({ embedThumbnail: event.target.checked })}
                />
                {sprich("Cover")}
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={settings.writeInfoJson}
                  onChange={(event) => patchSettings({ writeInfoJson: event.target.checked })}
                />
                {sprich("Metadaten-Datei")}
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={settings.writeDescription}
                  onChange={(event) => patchSettings({ writeDescription: event.target.checked })}
                />
                {sprich("Beschreibung")}
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={settings.downloadSubtitles}
                  onChange={(event) => patchSettings({ downloadSubtitles: event.target.checked })}
                />
                {sprich("Untertitel")}
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={settings.separateFolderPerVideo}
                  onChange={(event) =>
                    patchSettings({ separateFolderPerVideo: event.target.checked })
                  }
                />
                {sprich("Eigener Ordner pro Video")}
              </label>
            </div>

            <div className="grid-2" style={{ marginTop: 16 }}>
              <label className="field">
                <span>
                  {sprich("Gleichzeitige Downloads: {n}", { n: settings.maxParallelDownloads })}
                </span>
                <input
                  type="range"
                  min={1}
                  max={12}
                  value={settings.maxParallelDownloads}
                  onChange={(event) =>
                    patchSettings({ maxParallelDownloads: Number(event.target.value) })
                  }
                />
              </label>
              <label className="field">
                <span>
                  {sprich("Verbindungen pro Stream: {n}", { n: settings.aria2Connections })}
                </span>
                <input
                  type="range"
                  min={2}
                  max={16}
                  value={settings.aria2Connections}
                  onChange={(event) =>
                    patchSettings({ aria2Connections: Number(event.target.value) })
                  }
                />
              </label>
            </div>
          </div>
        )}
      </div>

      {presets.length > 0 && !result && !list && (
        <div className="tip-row">
          <Sparkles size={13} />
          <span>
            {sprich("Tipp: Über")} <strong>{sprich("Einstellungen")}</strong>{" "}
            {sprich("lässt sich alles anpassen — Presets, Zielordner, Metadaten und Tempo.")}
          </span>
        </div>
      )}
    </div>
  );
}
