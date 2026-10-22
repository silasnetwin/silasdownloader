import {
  Download,
  History,
  Info,
  ListVideo,
  Moon,
  Music4,
  Scissors,
  Settings,
  Sun,
} from "lucide-react";

import { useApp } from "./context";
import { useSprache, type Werte } from "../lib/i18n";

const PAGES = [
  { id: "home", label: "Start", icon: Download },
  { id: "downloads", label: "Downloads", icon: ListVideo },
  { id: "spotify", label: "Spotify", icon: Music4 },
  { id: "converter", label: "Konverter", icon: Scissors },
  { id: "history", label: "Verlauf", icon: History },
  { id: "settings", label: "Einstellungen", icon: Settings },
  { id: "about", label: "Über", icon: Info },
];

export default function Topbar() {
  const { page, setPage, theme, toggleTheme, engine, jobs } = useApp();
  const { sprich } = useSprache();
  const s = (deutsch: string, werte?: Werte) => sprich(deutsch, werte);

  const running = jobs.filter(
    (job) => job.state === "downloading" || job.state === "postProcessing",
  );
  const active = running.length;
  // Ein einziger Balken für die ganze Queue: der Anteil der erledigten Aufträge.
  const overall =
    jobs.length === 0
      ? 0
      : jobs.filter((job) => job.state === "completed").length / jobs.length;

  return (
    <>
      <header className="topbar">
        <button className="brand" onClick={() => setPage("home")}>
          <span className={`brand-mark${active > 0 ? " is-busy" : ""}`}>
            <img src="/brand-logo.svg" alt="" />
          </span>
          <span>SilasDownloader</span>
        </button>

        <nav className="top-nav">
          {PAGES.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              className={page === id ? "active" : ""}
              onClick={() => setPage(id)}
            >
              <Icon size={13} strokeWidth={2} />
              {s(label)}
            </button>
          ))}
        </nav>

        <div className="top-actions">
          <span className="engine-pill" title={`yt-dlp ${engine.ytdlp} · FFmpeg ${engine.ffmpeg}`}>
            <span className={`status-led${active > 0 ? " is-busy" : ""}`} />
            {active > 0 ? s("{n} aktiv", { n: active }) : s("bereit")}
          </span>
          <button
            className="icon-button"
            onClick={toggleTheme}
            title={theme === "dark" ? s("Helles Design") : s("Dunkles Design")}
          >
            {theme === "dark" ? <Sun size={15} /> : <Moon size={15} />}
          </button>
        </div>
      </header>

      {jobs.length > 0 && (
        <div className="queue-strip" title={s("{percent} % erledigt", { percent: Math.round(overall * 100) })}>
          <i style={{ width: `${Math.round(overall * 100)}%` }} />
        </div>
      )}
    </>
  );
}
