import { useApp } from "./context";
import { useSprache } from "../lib/i18n";

export default function Statusbar() {
  const { jobs, settings, engine } = useApp();
  const { sprich } = useSprache();
  const active = jobs.filter(
    (job) => job.state === "downloading" || job.state === "postProcessing",
  ).length;
  const done = jobs.filter((job) => job.state === "completed").length;
  const failed = jobs.filter((job) => job.state === "failed").length;

  return (
    <footer className="statusbar">
      <span>
        <span className={`status-led${active > 0 ? " is-busy" : ""}`} />
        {active > 0
          ? sprich(active === 1 ? "{n} Download laufen" : "{n} Downloads laufen", {
              n: active,
            })
          : sprich("Bereit")}
      </span>
      <span>
        {sprich(
          "{done} fertig · {failed} fehlgeschlagen · {parallel} parallel · yt-dlp {version}",
          {
            done,
            failed,
            parallel: settings.maxParallelDownloads,
            version: engine.ytdlp,
          },
        )}
      </span>
    </footer>
  );
}
