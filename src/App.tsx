import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { check } from "@tauri-apps/plugin-updater";

import * as api from "./lib/api";
import { uebersetze } from "./lib/i18n";
import type {
  DownloadRequest,
  HistoryEntry,
  ServerStatus,
  Settings,
} from "./lib/types";
import { AppContext, type Job, type Toast } from "./components/context";
import Topbar from "./components/Topbar";
import Statusbar from "./components/Statusbar";
import Home from "./pages/Home";
import Downloads from "./pages/Downloads";
import Converter from "./pages/Converter";
import SpotifyPage from "./pages/Spotify";
import HistoryPage from "./pages/History";
import SettingsPage from "./pages/Settings";
import About from "./pages/About";

const MAX_LOGS = 600;

/** Zeigt eine Meldung im Systembereich an.
 *
 * Wenn der Nutzer das nicht erlaubt hat, bleibt es bei der Meldung in der App.
 */
async function notifySystem(text: string) {
  try {
    const { isPermissionGranted, requestPermission, sendNotification } = await import(
      "@tauri-apps/plugin-notification"
    );
    let erlaubt = await isPermissionGranted();
    if (!erlaubt) {
      erlaubt = (await requestPermission()) === "granted";
    }
    if (erlaubt) {
      sendNotification({ title: "SilasDownloader", body: text });
    }
  } catch {
    // Ohne Systembenachrichtigung ist nichts zu tun.
  }
}

/** Liest die Zieladresse aus einer Adresse der Bauart `silasdownloader://`.
 *
 * Unterstützt werden `?url=…` (so schickt es die Browser-Erweiterung) und die
 * kurze Form `silasdownloader://https://…`.
 */
function linkTarget(raw: string): string | null {
  const text = (raw ?? "").trim();
  if (!text) return null;
  const frage = text.indexOf("?");
  if (frage >= 0) {
    const werte = new URLSearchParams(text.slice(frage + 1));
    const ziel = werte.get("url")?.trim();
    if (ziel) return ziel;
  }
  const ohne_schema = text.replace(/^silasdownloader:\/\//i, "").replace(/^\/+/, "");
  return /^https?:\/\//i.test(ohne_schema) ? ohne_schema : null;
}

export default function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [logs, setLogs] = useState<string[]>([]);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [page, setPage] = useState("home");
  const [engine, setEngine] = useState({ ytdlp: "…", ffmpeg: "…" });
  const [converter, setConverter] = useState({
    input: null as string | null,
    output: null as string | null,
    progress: 0,
    status: "Datei auswählen oder hineinziehen",
    running: false,
  });
  const [server, setServer] = useState<ServerStatus | null>(null);
  const nextLocalId = useRef(1);
  const jobsRef = useRef<Job[]>([]);
  const settingsRef = useRef<Settings | null>(null);
  const serverRef = useRef<ServerStatus | null>(null);

  /** Übersetzt einen deutschen Satz in die eingestellte Sprache.
   *
   * Die App selbst hält die Einstellungen — der Haken `useSprache` wäre hier
   * ein Kreislauf. Deshalb liest der Übersetzer die Sprache direkt.
   */
  const sprache = settings?.language === "en" ? "en" : "de";
  const sprich = useCallback(
    (deutsch: string, werte?: Record<string, string | number>) =>
      uebersetze(deutsch, sprache, werte),
    [sprache],
  );
  const sprichRef = useRef(sprich);
  useEffect(() => {
    sprichRef.current = sprich;
  }, [sprich]);

  // Die Ereignisse der Engine laufen in eigenen Funktionen. Damit sie immer die
  // aktuellen Werte sehen, liegen sie in Referenzen.
  useEffect(() => {
    settingsRef.current = settings;
  }, [settings]);
  useEffect(() => {
    serverRef.current = server;
  }, [server]);

  /* ---------- Start ---------- */

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [loadedSettings, loadedHistory] = await Promise.all([
          api.loadSettings(),
          api.loadHistory(),
        ]);
        if (cancelled) return;
        setSettings(loadedSettings);
        setHistory(loadedHistory);
        setPage(loadedSettings.startPage || "home");
      } catch (error) {
        console.error(error);
        notify(uebersetze("Einstellungen konnten nicht geladen werden.", "de"), "error");
      }
    })();
    api.engineVersions().then(([ytdlp, ffmpeg]) => setEngine({ ytdlp, ffmpeg }));
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /* ---------- Beim Anmelden starten ---------- */

  useEffect(() => {
    if (!settings) return;
    void import("@tauri-apps/plugin-autostart")
      .then(async ({ enable, disable, isEnabled }) => {
        const aktiv = await isEnabled();
        if (settings.autostart && !aktiv) await enable();
        if (!settings.autostart && aktiv) await disable();
      })
      .catch(() => {
        // Ohne die Erweiterung bleibt der Haken ohne Wirkung.
      });
  }, [settings?.autostart]);

  /* ---------- Dateien ins Fenster ziehen ---------- */

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type !== "drop") return;
      const path = event.payload.paths[0];
      if (!path) return;
      setConverter((current) => ({ ...current, input: path, progress: 0 }));
      setPage("converter");
    });
    return () => {
      void unlisten.then((off) => off());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /* ---------- Links aus dem Browser ins Fenster ziehen ---------- */

  useEffect(() => {
    // Der native Datei-Handler nimmt Dateien an. Für einen Link aus dem Browser
    // braucht es die Web-Ebene, sonst käme die Adresse nicht an.
    const over = (event: DragEvent) => {
      event.preventDefault();
    };
    const drop = (event: DragEvent) => {
      const text = event.dataTransfer?.getData("text/plain")?.trim() ?? "";
      if (!/^https?:\/\//i.test(text)) return;
      event.preventDefault();
      window.dispatchEvent(new CustomEvent("silas:link", { detail: text }));
    };
    window.addEventListener("dragover", over);
    window.addEventListener("drop", drop);
    return () => {
      window.removeEventListener("dragover", over);
      window.removeEventListener("drop", drop);
    };
  }, []);

  /* ---------- Adressen der Bauart silasdownloader:// ---------- */

  useEffect(() => {
    const unlisten = listen<string>("app://link", (event) => {
      const target = linkTarget(event.payload);
      if (!target) return;
      // Der Download beginnt auf der Startseite, also erst dorthin wechseln.
      setPage("home");
      window.setTimeout(() => {
        window.dispatchEvent(new CustomEvent("silas:link", { detail: target }));
      }, 120);
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, []);

  /* ---------- Fortschritt im Infobereich ---------- */

  const trayStep = useRef(-1);

  useEffect(() => {
    const laufend = jobs.filter(
      (job) => job.state === "downloading" || job.state === "postProcessing",
    );
    const fertig = jobs.filter((job) => job.state === "completed").length;
    const anteil = laufend.length
      ? laufend.reduce((summe, job) => summe + (job.progress || 0), 0) / laufend.length
      : 0;
    const schritt = Math.round(anteil * 100);
    if (schritt === trayStep.current) return;
    trayStep.current = schritt;
    void api
      .trayProgress(laufend.length, Math.max(jobs.length, fertig + laufend.length), anteil)
      .catch(() => undefined);
  }, [jobs]);

  /* ---------- Tresor des Servers ---------- */

  const refreshServer = useCallback(async () => {
    try {
      setServer(await api.serverStatus());
    } catch {
      setServer(null);
    }
  }, []);

  useEffect(() => {
    void refreshServer();
  }, [refreshServer]);

  /* ---------- Theme ---------- */
  const theme: "dark" | "light" = settings?.theme === "light" ? "light" : "dark";

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  /* ---------- Ereignisse der Engine ---------- */

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    const register = async () => {
      unlisteners.push(await api.onProgress((payload) => {
        const patch: Partial<Job> = {
          progress: payload.value,
          state: payload.state,
          status: payload.status,
        };
        if (payload.speed) patch.speed = payload.speed;
        if (payload.eta) patch.eta = payload.eta;
        if (payload.total) patch.total = payload.total;
        updateJob(payload.jobId, patch);
      }));
      unlisteners.push(await api.onTitle((payload) => {
        updateJob(payload.jobId, {
          title: payload.title,
          playlistIndex: payload.playlistIndex,
        });
      }));
      unlisteners.push(await api.onFile((payload) => {
        updateJob(payload.jobId, { outputFile: payload.path });
      }));
      unlisteners.push(await api.onLog((payload) => {
        if (payload.level === "debug") return;
        setLogs((current) => {
          const next = [...current, `${payload.level.toUpperCase()}  ${payload.line}`];
          return next.length > MAX_LOGS ? next.slice(next.length - MAX_LOGS) : next;
        });
      }));
      unlisteners.push(await api.onFinished((payload) => {
        const sag = sprichRef.current;
        updateJob(payload.jobId, {
          state: payload.ok ? "completed" : "failed",
          progress: payload.ok ? 1 : 0,
          status: payload.ok ? sag("Download abgeschlossen") : sag("Fehlgeschlagen"),
          outputFile: payload.outputFile,
          error: payload.error,
        });
        if (payload.technicalDetails) {
          setLogs((current) => [...current, payload.technicalDetails]);
        }
        void api.forgetJob(payload.jobId);
        if (payload.ok) {
          const file = payload.outputFile ?? payload.outputFiles[0] ?? null;
          // Fertige Dateien nach dem Download auf den Server legen. Wohin es geht,
          // entscheidet die Art des Inhalts: Film in die Filme, Serie in die Serien.
          const job = jobsRef.current.find((entry) => entry.id === payload.jobId);
          const kind = (job?.request as DownloadRequest | undefined)?.kind ?? "movie";
          if (file && settingsRef.current?.serverUpload && serverRef.current?.unlocked) {
            void api
              .serverUpload(file, kind)
              .then((bibliothek) => {
                if (!bibliothek) return;
                notify(sag("Auf den Server gelegt: {library}", { library: bibliothek }));
                void notifySystem(
                  sag("Auf den Server gelegt: {library}", { library: bibliothek }),
                );
              })
              .catch((error) =>
                notify(
                  sag("Der Upload ist fehlgeschlagen: {error}", { error: String(error) }),
                  "error",
                ),
              );
          }
          if (file && job) {
            const request = job.request as DownloadRequest | undefined;
            const mode = request?.settings.mode;
            const entry: HistoryEntry = {
              id: Date.now(),
              url: job.url,
              title: job.title,
              downloadedAt: new Date().toISOString(),
              format:
                mode === "audio"
                  ? (request?.settings.audioFormat ?? "audio").toUpperCase()
                  : (request?.settings.videoContainer ?? "mp4").toUpperCase(),
              quality:
                mode === "audio"
                  ? "Audio"
                  : (request?.settings.videoQuality ?? "").toUpperCase(),
              outputFile: file,
              success: true,
            };
            setHistory((current) => {
              const next = [...current, entry];
              void api.saveHistory(next);
              return next;
            });
          }
          notify(sag("Download abgeschlossen."));
          void notifySystem(sag("Download abgeschlossen"));
        } else {
          // Ein Ausrutscher (etwa ein 403 vom CDN bei vielen parallelen Streams)
          // soll den Auftrag nicht gleich beenden. Er wird automatisch wiederholt.
          const job = jobsRef.current.find((entry) => entry.id === payload.jobId);
          if (job && job.attempts < 2) {
            const next = job.attempts + 1;
            updateJob(payload.jobId, {
              attempts: next,
              status: sag("Fehlgeschlagen – Wiederholung {n}/2 läuft …", { n: next }),
            });
            window.setTimeout(() => {
              void api
                .startDownload(job.request as DownloadRequest)
                .catch(() => undefined);
            }, 6000);
          }
          notify(payload.error ?? sag("Der Download ist fehlgeschlagen."), "error");
        }
      }));
      unlisteners.push(
        await listen<{ value: number; status: string }>("convert://progress", (event) => {
          setConverter((current) => ({
            ...current,
            progress: event.payload.value,
            status: event.payload.status,
          }));
        }),
      );
    };
    void register();
    return () => {
      unlisteners.forEach((off) => off());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /* ---------- Meldungen ---------- */

  const notify = useCallback((text: string, kind: "info" | "error" = "info") => {
    const id = Date.now() + Math.random();
    setToasts((current) => [...current, { id, text, kind }]);
    window.setTimeout(() => {
      setToasts((current) => current.filter((toast) => toast.id !== id));
    }, 4200);
  }, []);

  const dismissToast = useCallback((id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);

  /* ---------- Updates ---------- */

  // Beim Start wird einmal leise nach einer neuen Version gefragt. Gibt es eine,
  // erscheint ein Hinweis; installiert wird sie im Tab „Über". Ohne Netz bleibt
  // es still — ein fehlender Server darf den Start nicht stören.
  useEffect(() => {
    const timer = window.setTimeout(() => {
      void check()
        .then((update) => {
          if (!update) return;
          notify(
            sprichRef.current("Version {version} ist verfügbar — im Tab „Über\" installieren.", {
              version: update.version,
            }),
          );
          void notifySystem(
            sprichRef.current("Update verfügbar: Version {version}", {
              version: update.version,
            }),
          );
        })
        .catch(() => {
          // Kein Netz, kein Update — kein Grund für eine Meldung.
        });
    }, 10_000);
    return () => window.clearTimeout(timer);
  }, [notify]);

  /* ---------- Jobs ---------- */

  useEffect(() => {
    jobsRef.current = jobs;
  }, [jobs]);

  const addJob = useCallback(
    (
      job: Omit<
        Job,
        "id" | "state" | "progress" | "status" | "speed" | "eta" | "total" | "outputFile" | "error"
      >,
    ) => {
      const id = nextLocalId.current++;
      setJobs((current) => [
        ...current,
        {
          ...job,
          id,
          state: "pending",
          progress: 0,
          status: sprich("Bereit"),
          speed: "",
          eta: "",
          total: "",
          outputFile: null,
          error: null,
        },
      ]);
      return id;
    },
    [sprich],
  );

  const updateJob = useCallback((id: number, patch: Partial<Job>) => {
    setJobs((current) =>
      current.map((job) => (job.id === id ? { ...job, ...patch } : job)),
    );
  }, []);

  /* ---------- Queue: startet so viele Aufträge wie erlaubt ---------- */

  const enqueue = useCallback(
    (job: {
      url: string;
      title: string;
      request: DownloadRequest;
      playlistIndex?: number | null;
    }) => {
      const id = nextLocalId.current++;
      setJobs((current) => [
        ...current,
        {
          id,
          url: job.url,
          title: job.title,
          state: "pending",
          progress: 0,
          status: sprich("Bereit"),
          speed: "",
          eta: "",
          total: "",
          outputFile: null,
          error: null,
          attempts: 0,
          playlistIndex: job.playlistIndex ?? null,
          request: job.request,
        },
      ]);
    },
    [sprich],
  );

  useEffect(() => {
    const limit = Math.max(1, Math.min(settings?.maxParallelDownloads ?? 6, 12));
    const running = jobs.filter(
      (job) =>
        job.state === "analyzing" ||
        job.state === "downloading" ||
        job.state === "postProcessing",
    ).length;
    if (running >= limit) return;
    const next = jobs.find((job) => job.state === "pending");
    if (!next) return;
    updateJob(next.id, { state: "analyzing", status: sprich("Download wird vorbereitet") });
    api.startDownload(next.request as DownloadRequest).catch((error) => {
      updateJob(next.id, {
        state: "failed",
        error: String(error),
        status: sprich("Fehlgeschlagen"),
      });
    });
  }, [jobs, settings?.maxParallelDownloads, updateJob, sprich]);

  const removeJob = useCallback((id: number) => {
    setJobs((current) => current.filter((job) => job.id !== id));
  }, []);

  const cancelJob = useCallback(
    (id: number) => {
      void api.cancelDownload(id);
      updateJob(id, { status: sprich("Wird abgebrochen …") });
    },
    [updateJob, sprich],
  );

  const retryJob = useCallback(
    (id: number) => {
      const job = jobs.find((entry) => entry.id === id);
      if (!job) return;
      updateJob(id, {
        state: "pending",
        progress: 0,
        error: null,
        attempts: 0,
        status: sprich("Bereit für neuen Versuch"),
      });
      void api.startDownload(job.request as never);
    },
    [jobs, updateJob, sprich],
  );

  /* ---------- Einstellungen ---------- */

  const patchSettings = useCallback((patch: Partial<Settings>) => {
    setSettings((current) => {
      if (!current) return current;
      const next = { ...current, ...patch };
      void api.saveSettings(next);
      return next;
    });
  }, []);

  const pushLog = useCallback((line: string) => {
    setLogs((current) => {
      const next = [...current, line];
      return next.length > MAX_LOGS ? next.slice(next.length - MAX_LOGS) : next;
    });
  }, []);

  const clearLogs = useCallback(() => setLogs([]), []);

  const value = useMemo(
    () => ({
      settings: settings ?? ({} as Settings),
      patchSettings,
      history,
      setHistory,
      jobs,
      addJob,
      enqueue,
      updateJob,
      removeJob,
      retryJob,
      cancelJob,
      logs,
      pushLog,
      clearLogs,
      toasts,
      notify,
      dismissToast,
      theme,
      toggleTheme: () =>
        patchSettings({ theme: theme === "dark" ? "light" : "dark" }),
      engine,
      page,
      setPage,
      converter,
      setConverterInput: (path: string | null) =>
        setConverter((current) => ({ ...current, input: path, progress: 0 })),
      setConverterOutput: (path: string | null) =>
        setConverter((current) => ({ ...current, output: path })),
      server,
      refreshServer,
    }),
    [
      settings,
      patchSettings,
      history,
      jobs,
      addJob,
      enqueue,
      updateJob,
      removeJob,
      retryJob,
      cancelJob,
      logs,
      pushLog,
      clearLogs,
      toasts,
      notify,
      dismissToast,
      theme,
      engine,
      page,
      server,
      refreshServer,
    ],
  );

  if (!settings) {
    return (
      <div className="app-shell">
        <div className="page">
          <div className="page-inner">
            <div className="loading-row" style={{ paddingTop: 40 }}>
              <span className="spinner" />
              {sprich("SilasDownloader wird gestartet …")}
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <AppContext.Provider value={value}>
      <div className="app-shell">
        <Topbar />
        {settings ? (
          <div className="page">
            {page === "home" && <Home />}
            {page === "downloads" && <Downloads />}
            {page === "spotify" && <SpotifyPage />}
            {page === "converter" && <Converter />}
            {page === "history" && <HistoryPage />}
            {page === "settings" && <SettingsPage />}
            {page === "about" && <About />}
          </div>
        ) : (
          <div className="page" />
        )}
        <Statusbar />
        <div className="toast-stack">
          {toasts.map((toast) => (
            <div
              key={toast.id}
              className={`toast${toast.kind === "error" ? " is-error" : ""}`}
              onClick={() => dismissToast(toast.id)}
            >
              {toast.text}
            </div>
          ))}
        </div>
      </div>
    </AppContext.Provider>
  );
}
