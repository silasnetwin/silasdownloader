// Füllt das Feld mit der aktuellen Seite und reicht sie an die App weiter.

const field = document.getElementById("url");
const state = document.getElementById("state");

/** Baut die Adresse der App. */
function appUrl(target) {
  return `silasdownloader://download?url=${encodeURIComponent(target)}`;
}

/** Übergibt eine Adresse über den kurzen Umweg durch einen Tab. */
function openInApp(target) {
  if (!/^https?:\/\//i.test(target)) {
    state.textContent = "Bitte eine Adresse mit http:// oder https:// eingeben.";
    return;
  }
  chrome.tabs.create({ url: appUrl(target), active: false }, (tab) => {
    if (tab?.id !== undefined) {
      setTimeout(() => chrome.tabs.remove(tab.id).catch(() => {}), 1500);
    }
    state.textContent = "Übergeben. Windows fragt beim ersten Mal nach.";
  });
}

// Die aktuelle Seite als Vorschlag eintragen.
chrome.tabs
  .query({ active: true, currentWindow: true })
  .then(([tab]) => {
    if (tab?.url) field.value = tab.url;
  })
  .catch(() => {});

document.getElementById("send").addEventListener("click", () => {
  openInApp(field.value.trim());
});

field.addEventListener("keydown", (event) => {
  if (event.key === "Enter") openInApp(field.value.trim());
});
