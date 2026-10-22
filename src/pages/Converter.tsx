import { useState } from "react";
import { FileVideo, FolderOpen, Scissors } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";

import { useApp } from "../components/context";
import * as api from "../lib/api";
import { useSprache } from "../lib/i18n";
import type { ConvertFormat } from "../lib/types";

const FORMATS: Array<{ value: ConvertFormat; hint: string }> = [
  { value: "mp4", hint: "H.264, mit Schnellstart" },
  { value: "mkv", hint: "ohne Neucodierung" },
  { value: "webm", hint: "VP9 / Opus" },
  { value: "mov", hint: "H.264, mit Schnellstart" },
  { value: "mp3", hint: "256 kbit/s" },
  { value: "m4a", hint: "AAC, 256 kbit/s" },
  { value: "wav", hint: "unverändert" },
  { value: "flac", hint: "verlustfrei" },
];

export default function Converter() {
  const { converter, setConverterInput, setConverterOutput, notify } = useApp();
  const { sprich } = useSprache();
  const [format, setFormat] = useState<ConvertFormat>("mp4");
  const [busy, setBusy] = useState(false);

  const chooseInput = async () => {
    const picked = await open({
      multiple: false,
      filters: [
        {
          name: sprich("Medien"),
          extensions: ["mp4", "mkv", "webm", "mov", "avi", "mp3", "m4a", "wav", "flac", "ts"],
        },
      ],
    });
    if (typeof picked === "string") setConverterInput(picked);
  };

  const convert = async () => {
    if (!converter.input) {
      notify(sprich("Bitte zuerst eine Datei auswählen."), "error");
      return;
    }
    setBusy(true);
    try {
      const result = await api.convertMedia(converter.input, format);
      setConverterOutput(result);
      notify(sprich("Konvertierung abgeschlossen."));
    } catch (error) {
      notify(String(error), "error");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="page-inner">
      <div className="page-head">
        <div className="eyebrow">
          <span className="eyebrow-line" />
          {sprich("KONVERTER")}
        </div>
        <h1>{sprich("Media-Konverter")}</h1>
        <p>
          {sprich(
            "Videos und Audio lokal mit FFmpeg umwandeln — ohne Upload und ohne Internet.",
          )}
        </p>
      </div>

      <div className="card">
        <div className="card-head">
          <div>
            <h2>{sprich("Quelldatei")}</h2>
            <p>{sprich("Datei auswählen oder in das Fenster ziehen.")}</p>
          </div>
          <button className="primary-button" onClick={chooseInput}>
            <FileVideo size={13} /> {sprich("Datei wählen")}
          </button>
        </div>

        {converter.input ? (
          <div className="meta-grid">
            <div className="meta-cell" style={{ gridColumn: "1 / -1" }}>
              <span>{sprich("Datei")}</span>
              <b
                style={{
                  display: "block",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {converter.input}
              </b>
            </div>
          </div>
        ) : (
          <div className="empty">
            <b>{sprich("Noch keine Datei gewählt.")}</b>
            <small>
              {sprich("Ziehe eine Datei hierher oder nutze die Schaltfläche oben.")}
            </small>
          </div>
        )}
      </div>

      <div className="card">
        <div className="card-head">
          <div>
            <h2>{sprich("Zielformat")}</h2>
            <p>
              {sprich(
                "MP4 und MOV werden neu kodiert, MKV wird unverändert kopiert.",
              )}
            </p>
          </div>
        </div>
        <div className="switch-row">
          {FORMATS.map((entry) => (
            <button
              key={entry.value}
              className={`chip${format === entry.value ? " is-active" : ""}`}
              onClick={() => setFormat(entry.value)}
              title={sprich(entry.hint)}
            >
              {entry.value.toUpperCase()}
            </button>
          ))}
        </div>
        <div className="job-actions" style={{ marginTop: 16 }}>
          <button
            className="primary-button"
            onClick={convert}
            disabled={!converter.input || busy}
          >
            <Scissors size={13} /> {busy ? sprich("Konvertiert …") : sprich("Konvertieren")}
          </button>
          {converter.output && (
            <button
              className="secondary-button"
              onClick={() => void api.revealPath(converter.output as string)}
            >
              <FolderOpen size={13} /> {sprich("Ergebnis zeigen")}
            </button>
          )}
        </div>

        {converter.running || converter.progress > 0 ? (
          <>
            <div className="progress" style={{ marginTop: 16 }}>
              <i style={{ width: `${Math.round(converter.progress * 100)}%` }} />
            </div>
            <div className="job-meta">
              <span>{converter.status}</span>
            </div>
          </>
        ) : null}
      </div>
    </div>
  );
}
