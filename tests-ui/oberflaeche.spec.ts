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

type EngineCall = { command: string; args: Record<string, unknown> };

/** Die Aufrufe, die die Oberfläche an die Engine geschickt hat. */
async function engineCalls(page: Page): Promise<EngineCall[]> {
  return page.evaluate(() => {
    const bridge = (
      window as unknown as { __silasE2E?: { calls: EngineCall[] } }
    ).__silasE2E;
    return bridge?.calls ?? [];
  });
}

/** Schickt ein Ereignis, als käme es aus der Rust-Seite. */
async function emitEngine(page: Page, event: string, payload: unknown) {
  await page.evaluate(
    ({ event, payload }) => {
      const bridge = (
        window as unknown as {
          __silasE2E?: { emit: (name: string, data: unknown) => Promise<void> };
        }
      ).__silasE2E;
      return bridge?.emit(event, payload);
    },
    { event, payload },
  );
}

/** Füllt einen Link ein und startet den Download des Testvideos. */
async function starteTestvideo(page: Page) {
  await page.getByPlaceholder(/Link/i).first().fill("https://example.com/video/1");
  await page.getByRole("button", { name: /Holen/ }).click();
  await expect(page.getByRole("heading", { name: "Testvideo" })).toBeVisible();
  await page.getByRole("button", { name: /Herunterladen/ }).click();
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

test("startet einen Download mit der Nummer aus der Oberfläche", async ({ page }) => {
  await oeffne(page);
  await starteTestvideo(page);

  await expect
    .poll(
      async () =>
        (await engineCalls(page)).filter((call) => call.command === "start_download")
          .length,
    )
    .toBe(1);

  const call = (await engineCalls(page)).find(
    (entry) => entry.command === "start_download",
  );
  // Die Engine bekommt genau die Nummer, unter der die Oberfläche den Auftrag
  // führt. Sonst käme der Fortschritt nie bei der richtigen Zeile an.
  expect(call?.args.jobId).toBe(1);
  expect((call?.args.request as { url: string }).url).toBe(
    "https://example.com/video/1",
  );
});

test("wiederholt einen Auftrag unter derselben Nummer", async ({ page }) => {
  await oeffne(page);
  await starteTestvideo(page);

  // Der erste Versuch scheitert.
  await emitEngine(page, "download://finished", {
    jobId: 1,
    ok: false,
    error: "Netzwerk weg",
    technicalDetails: "",
    outputFile: null,
    outputFiles: [],
  });
  await expect(page.getByText(/Fehlgeschlagen/).first()).toBeVisible();

  await page.getByRole("button", { name: /Erneut versuchen/ }).click();

  // Genau ein neuer Versuch, und zwar unter derselben Nummer.
  await expect
    .poll(
      async () =>
        (await engineCalls(page)).filter((call) => call.command === "start_download")
          .length,
    )
    .toBe(2);
  const starts = (await engineCalls(page)).filter(
    (call) => call.command === "start_download",
  );
  expect(starts.map((call) => call.args.jobId)).toEqual([1, 1]);

  // Der zweite Versuch klappt — die Meldung muss wieder bei der Zeile landen.
  await emitEngine(page, "download://finished", {
    jobId: 1,
    ok: true,
    error: null,
    technicalDetails: "",
    outputFile: "C:\\Downloads\\Testvideo.mp4",
    outputFiles: ["C:\\Downloads\\Testvideo.mp4"],
  });
  await expect(page.getByText("Fertig", { exact: true }).first()).toBeVisible();
});
