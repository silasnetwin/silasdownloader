// Legt die Adresse der App an und reicht sie an Windows weiter.
//
// Windows fragt beim ersten Mal nach, ob SilasDownloader die Adresse öffnen
// darf. Danach läuft es ohne Nachfrage.

const SCHEMA = "silasdownloader";

/** Baut die Adresse der App für einen Link. */
function appUrl(target) {
  return `${SCHEMA}://download?url=${encodeURIComponent(target)}`;
}

/** Öffnet die Adresse über einen kurzen Tab, der sofort wieder verschwindet. */
function handOver(target) {
  if (!/^https?:\/\//i.test(target)) return;
  chrome.tabs.create({ url: appUrl(target), active: false }, (tab) => {
    if (tab?.id === undefined) return;
    // Dem Betriebssystem bleibt genug Zeit, um die App zu starten.
    setTimeout(() => chrome.tabs.remove(tab.id).catch(() => {}), 1500);
  });
}

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: "silas-link",
      title: "Mit SilasDownloader laden",
      contexts: ["link"],
    });
    chrome.contextMenus.create({
      id: "silas-page",
      title: "Diese Seite mit SilasDownloader laden",
      contexts: ["page"],
    });
  });
});

chrome.contextMenus.onClicked.addListener((info) => {
  const target = info.linkUrl ?? info.pageUrl ?? "";
  handOver(target);
});

// Die Oberfläche darf auch aus dem Fenster heraus an die App übergeben.
chrome.runtime.onMessage.addListener((message) => {
  if (message?.type === "silas-download") {
    handOver(String(message.url ?? ""));
  }
});
