//! Die Sprache der Oberfläche.
//!
//! Die App ist ursprünglich auf Deutsch geschrieben. Der Schlüssel eines Textes
//! ist deshalb der deutsche Satz selbst: fehlt eine Übersetzung, zeigt die App
//! den deutschen Text. So kann beim Umbau nichts kaputtgehen.
//!
//! Neue Texte gehören in das Wörterbuch `en.ts`. Fehlt dort ein Eintrag, bleibt
//! der deutsche Satz stehen — die Oberfläche ist damit nie unvollständig.

import { useApp } from "../components/context";
import type { Language } from "./types";
import { en } from "./en";

/** Ersetzt Platzhalter der Form `{name}`. */
export type Werte = Record<string, string | number>;

/**
 * Übersetzt einen deutschen Satz in die gewählte Sprache.
 *
 * Platzhalter werden über `werte` gefüllt: `sprich("Noch {n} Sekunden", { n: 5 })`.
 */
export function uebersetze(deutsch: string, sprache: Language, werte?: Werte): string {
  const satz = sprache === "en" ? (en[deutsch] ?? deutsch) : deutsch;
  if (!werte) return satz;
  return satz.replace(/\{(\w+)\}/g, (treffer, name: string) =>
    werte[name] === undefined ? treffer : String(werte[name]),
  );
}

/** Der Übersetzer für die Oberfläche. */
export function useSprache() {
  const { settings } = useApp();
  const sprache: Language = settings.language === "en" ? "en" : "de";
  return {
    sprache,
    /** Übersetzt einen deutschen Satz. */
    sprich: (deutsch: string, werte?: Werte) => uebersetze(deutsch, sprache, werte),
  };
}
