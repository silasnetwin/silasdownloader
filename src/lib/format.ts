/** Formatierungshelfer für die Oberfläche. */

export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let n = bytes;
  let i = 0;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i += 1;
  }
  return i === 0 ? `${bytes} ${units[i]}` : `${n.toFixed(1)} ${units[i]}`;
}

export function formatDuration(seconds: number | null): string {
  if (seconds === null) return "–";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  return h > 0
    ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`
    : `${m}:${String(s).padStart(2, "0")}`;
}

export function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "–";
  return date.toLocaleString("de-DE", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function formatUploadDate(raw: string): string {
  if (!raw || raw.length !== 8) return raw || "–";
  return `${raw.slice(6, 8)}.${raw.slice(4, 6)}.${raw.slice(0, 4)}`;
}

/** Zerlegt einen Text in eine Playlist-Auswahl wie `1-3,5,7-9`. */
export function playlistItemsSpec(indices: number[]): string {
  const sorted = [...new Set(indices)].sort((a, b) => a - b);
  const parts: string[] = [];
  let start: number | null = null;
  let previous: number | null = null;
  for (const index of sorted) {
    if (start !== null && previous !== null && index === previous + 1) {
      previous = index;
      continue;
    }
    if (start !== null && previous !== null) {
      parts.push(start === previous ? `${start}` : `${start}-${previous}`);
    }
    start = index;
    previous = index;
  }
  if (start !== null && previous !== null) {
    parts.push(start === previous ? `${start}` : `${start}-${previous}`);
  }
  return parts.join(",");
}

/** Sammelt alle Links aus einem Text. */
export function collectUrls(input: string): string[] {
  const found: string[] = [];
  const pattern = /https?:\/\/[^\s"'()<>\]]+/gi;
  for (const match of input.matchAll(pattern)) {
    const raw = match[0].replace(/[.,;]+$/, "");
    if (!/^https?:\/\/[^\s/]+/i.test(raw)) continue;
    if (!found.includes(raw)) found.push(raw);
  }
  return found;
}

/** Kürzt einen Text für die Anzeige. */
export function truncate(text: string, max = 68): string {
  if (text.length <= max) return text;
  return `${text.slice(0, max - 1)}…`;
}
