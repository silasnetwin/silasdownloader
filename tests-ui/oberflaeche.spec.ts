import { expect, test, type Page } from "@playwright/test";

/** Öffnet die App mit dem Prüfstand anstelle der Rust-Seite. */
async function oeffne(page: Page) {
  await page.goto("/?e2e=1");
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
}

/** Wechselt über die Navigation auf eine Seite. */
async function gehe(page: Page, name: string) {
  await page.getByRole("button", { name, exact: true }).click();
}

test("startet auf der Startseite", async ({ page }) => {
  await oeffne(page);
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Alles");
  await expect(page.getByPlaceholder(/Link/i).first()).toBeVisible();
  await expect(page.getByRole("button", { name: /Holen/ })).toBeVisible();
  // Die Spalte der Aufträge ist zu sehen.
  await expect(page.getByText(/Bereit/).first()).toBeVisible();
});

test("zeigt alle Abschnitte der Einstellungen", async ({ page }) => {
  await oeffne(page);
  await gehe(page, "Einstellungen");
  for (const titel of [
    "App & Workflow",
    "Eigene Presets",
    "Geschwindigkeit & Queue",
    "Metadaten & Zusatzdateien",
    "Engine & Updates",
    "Server-Upload",
  ]) {
    await expect(page.getByRole("heading", { name: titel })).toBeVisible();
  }
});

test("schaltet den Server-Upload ein und speichert die Angaben", async ({ page }) => {
  await oeffne(page);
  await gehe(page, "Einstellungen");

  // Der Abschnitt steht eingeklappt ganz unten.
  await page.getByRole("button", { name: /Server-Upload/ }).click();

  const passwort = page.getByPlaceholder("Eigenes Passwort wählen");
  await passwort.fill("testpasswort");
  await page.getByLabel("Tailscale-Adresse").fill("100.100.1.1");
  await page.getByLabel("Name im SSH-Config").fill("mein-server");
  await page.getByRole("button", { name: /Tresor anlegen/ }).click();

  await expect(page.getByText(/Tresor angelegt|Gespeichert/).first()).toBeVisible();
});

test("schaltet die Startoptionen in den Einstellungen", async ({ page }) => {
  await oeffne(page);
  await gehe(page, "Einstellungen");

  const tray = page.getByLabel("Schließen legt die App in den Infobereich");
  const autostart = page.getByLabel("Beim Anmelden starten (im Hintergrund)");
  await expect(tray).toBeChecked();
  await expect(autostart).not.toBeChecked();
  await autostart.check();
  await expect(autostart).toBeChecked();
});

test("zeigt die Version im Tab Über", async ({ page }) => {
  await oeffne(page);
  await gehe(page, "Über");
  await expect(page.getByText("1.3.0").first()).toBeVisible();
});

test("schaltet die Oberfläche auf Englisch", async ({ page }) => {
  await oeffne(page);
  await gehe(page, "Einstellungen");

  // Die Sprache steht ganz oben im Abschnitt „App & Workflow".
  await page.getByLabel(/^Sprache/).first().selectOption("en");

  // Die Navigation ist sofort englisch.
  await expect(page.getByRole("button", { name: "Settings", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();

  // Der Server-Upload heißt auf Englisch „Server upload".
  await expect(page.getByRole("heading", { name: "Server upload" })).toBeVisible();

  // Zurück auf Deutsch.
  await page.getByLabel(/^Language/).first().selectOption("de");
  await expect(page.getByRole("heading", { name: "Einstellungen" })).toBeVisible();
});
