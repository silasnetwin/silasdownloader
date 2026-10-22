import { FolderOpen, Trash2 } from "lucide-react";

import { useApp } from "../components/context";
import { revealPath, saveHistory } from "../lib/api";
import { formatDate } from "../lib/format";
import { useSprache } from "../lib/i18n";

export default function HistoryPage() {
  const { history, setHistory, notify } = useApp();
  const { sprich } = useSprache();

  const clear = async () => {
    setHistory([]);
    await saveHistory([]);
    notify(sprich("Verlauf gelöscht."));
  };

  return (
    <div className="page-inner">
      <div className="page-head">
        <div className="eyebrow">
          <span className="eyebrow-line" />
          {sprich("VERLAUF")}
        </div>
        <h1>{sprich("Verlauf")}</h1>
        <p>{sprich("Alle abgeschlossenen Downloads dieses Geräts.")}</p>
      </div>

      <div className="card">
        <div className="card-head">
          <div>
            <h2>{sprich("Einträge")}</h2>
            <p>{sprich("{n} Downloads", { n: history.length })}</p>
          </div>
          {history.length > 0 && (
            <button className="ghost-button" onClick={() => void clear()}>
              <Trash2 size={12} /> {sprich("Verlauf löschen")}
            </button>
          )}
        </div>

        {history.length === 0 ? (
          <div className="empty">
            <b>{sprich("Noch keine Downloads.")}</b>
            <small>
              {sprich("Abgeschlossene Downloads erscheinen hier automatisch.")}
            </small>
          </div>
        ) : (
          <>
            <div className="history-row history-head">
              <span>{sprich("Datei")}</span>
              <span>{sprich("Format")}</span>
              <span>{sprich("Datum")}</span>
              <span />
            </div>
            {[...history].reverse().map((entry) => (
              <div className="history-row" key={entry.id}>
                <span title={entry.outputFile}>{entry.title || entry.url}</span>
                <span style={{ color: "var(--muted)" }}>{entry.format || "–"}</span>
                <span style={{ color: "var(--muted)" }}>
                  {formatDate(entry.downloadedAt)}
                </span>
                <span style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
                  <button
                    className="ghost-button"
                    onClick={() =>
                      void revealPath(entry.outputFile).catch((error) =>
                        notify(String(error), "error"),
                      )
                    }
                  >
                    <FolderOpen size={12} /> {sprich("Öffnen")}
                  </button>
                </span>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}
