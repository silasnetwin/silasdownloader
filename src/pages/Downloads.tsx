import { useState } from "react";
import { ListVideo, Trash2 } from "lucide-react";

import { useApp } from "../components/context";
import JobRow from "../components/JobRow";
import { useSprache } from "../lib/i18n";

export default function Downloads() {
  const { jobs, removeJob, logs, clearLogs } = useApp();
  const { sprich } = useSprache();
  const [showLog, setShowLog] = useState(false);
  const [onlyActive, setOnlyActive] = useState(false);

  const pending = jobs.filter((job) => job.state === "pending").length;
  const active = jobs.filter(
    (job) => job.state === "downloading" || job.state === "postProcessing",
  ).length;
  const done = jobs.filter((job) => job.state === "completed").length;

  const visible = onlyActive
    ? jobs.filter(
        (job) =>
          job.state === "downloading" ||
          job.state === "postProcessing" ||
          job.state === "pending",
      )
    : jobs;

  return (
    <div className="page-inner">
      <div className="page-head">
        <div className="eyebrow">
          <span className="eyebrow-line" />
          {sprich("QUEUE")}
        </div>
        <h1>{sprich("Downloads")}</h1>
        <p>
          {sprich(
            "Hier siehst du exakt, was gerade passiert und wo die Datei landet.",
          )}
        </p>
      </div>

      <div className="card">
        <div className="card-head">
          <div>
            <h2>{sprich("Aktive Aufträge")}</h2>
            <p>
              {sprich("{pending} wartend · {active} aktiv · {done} fertig", {
                pending,
                active,
                done,
              })}
            </p>
          </div>
          <div className="job-actions" style={{ marginTop: 0 }}>
            {jobs.length > 6 && (
              <button
                className={`chip${onlyActive ? " is-active" : ""}`}
                onClick={() => setOnlyActive((value) => !value)}
              >
                {sprich("Nur offene")}
              </button>
            )}
            <button className="ghost-button" onClick={() => setShowLog((v) => !v)}>
              <ListVideo size={12} /> {sprich("Protokoll")}
            </button>
            <button
              className="ghost-button"
              onClick={() =>
                jobs
                  .filter((job) => job.state !== "downloading" && job.state !== "postProcessing")
                  .forEach((job) => removeJob(job.id))
              }
            >
              <Trash2 size={12} /> {sprich("Fertige entfernen")}
            </button>
          </div>
        </div>

        {jobs.length === 0 ? (
          <div className="empty">
            <b>{sprich("Keine Downloads in der Queue.")}</b>
            <small>
              {sprich("Gehe auf Home, füge einen Link ein und starte den Download.")}
            </small>
          </div>
        ) : visible.length === 0 ? (
          <div className="empty">
            <b>{sprich("Keine offenen Aufträge.")}</b>
            <small>
              {sprich(
                "Alle Downloads sind durch. Blende den Filter aus, um fertige zu sehen.",
              )}
            </small>
          </div>
        ) : (
          visible.map((job) => <JobRow key={job.id} job={job} />)
        )}
      </div>

      {showLog && (
        <div className="card">
          <div className="card-head">
            <div>
              <h2>{sprich("Protokoll")}</h2>
              <p>{sprich("Die Ausgabe der Download-Engine.")}</p>
            </div>
            <button className="ghost-button" onClick={clearLogs}>
              {sprich("Leeren")}
            </button>
          </div>
          <div className="log">
            {logs.length === 0 ? sprich("Noch keine Einträge.") : logs.join("\n")}
          </div>
        </div>
      )}
    </div>
  );
}
