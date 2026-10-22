import { FolderOpen, RotateCw, Trash2, X } from "lucide-react";

import { useApp } from "./context";
import { revealPath } from "../lib/api";
import { useSprache } from "../lib/i18n";
import type { JobState } from "../lib/types";

const STATE_LABEL: Record<JobState, string> = {
  pending: "Wartet",
  analyzing: "Analysiert",
  downloading: "Download",
  postProcessing: "Verarbeitung",
  completed: "Fertig",
  failed: "Fehlgeschlagen",
  cancelled: "Abgebrochen",
};

function badgeClass(state: JobState): string {
  if (state === "completed") return "job-badge is-done";
  if (state === "failed") return "job-badge is-error";
  if (state === "downloading" || state === "postProcessing") return "job-badge is-busy";
  return "job-badge";
}

export default function JobRow({ job }: { job: import("./context").Job }) {
  const { cancelJob, retryJob, removeJob, notify } = useApp();
  const { sprich } = useSprache();
  const running =
    job.state === "downloading" ||
    job.state === "postProcessing" ||
    job.state === "analyzing";

  return (
    <div className="job">
      <div className="job-head">
        <span className={badgeClass(job.state)}>{sprich(STATE_LABEL[job.state])}</span>
        <span className="job-title">{job.title}</span>
      </div>
      <div className="job-url">{job.url}</div>

      <div className="progress">
        <i style={{ width: `${Math.round(Math.min(job.progress, 1) * 100)}%` }} />
      </div>

      <div className="job-meta">
        <span>{job.status}</span>
        {job.speed && <span>{sprich("Speed {value}", { value: job.speed })}</span>}
        {job.eta && <span>{sprich("ETA {value}", { value: job.eta })}</span>}
        {job.total && <span>{sprich("Größe {value}", { value: job.total })}</span>}
        {job.playlistIndex !== null && <span>#{job.playlistIndex}</span>}
      </div>

      {job.error && (
        <div className="notice is-error">
          <span>
            <strong>{sprich("Fehlgeschlagen:")}</strong> {job.error}
            {job.attempts > 0 && sprich("(automatisch wiederholt: {n}/2)", { n: job.attempts })}
          </span>
        </div>
      )}

      <div className="job-actions">
        {running && (
          <button className="ghost-button" onClick={() => cancelJob(job.id)}>
            <X size={12} /> {sprich("Abbrechen")}
          </button>
        )}
        {(job.state === "failed" || job.state === "cancelled") && (
          <button className="ghost-button" onClick={() => retryJob(job.id)}>
            <RotateCw size={12} /> {sprich("Erneut versuchen")}
          </button>
        )}
        {job.outputFile && (
          <button
            className="ghost-button"
            onClick={() => {
              void revealPath(job.outputFile as string).catch((error) =>
                notify(String(error), "error"),
              );
            }}
          >
            <FolderOpen size={12} /> {sprich("Im Explorer zeigen")}
          </button>
        )}
        {!running && (
          <button className="ghost-button" onClick={() => removeJob(job.id)}>
            <Trash2 size={12} /> {sprich("Entfernen")}
          </button>
        )}
      </div>
    </div>
  );
}
