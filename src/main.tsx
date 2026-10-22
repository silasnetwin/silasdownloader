import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import App from "./App";
import "./styles/tokens.css";
import "./styles/app.css";

const container = document.getElementById("root");
if (!container) throw new Error("Wurzel-Element nicht gefunden");

// Der Prüfstand der Oberfläche wird nur im Entwicklungsmodus und nur auf
// ausdrücklichen Wunsch geladen (`?e2e=1`).
if (import.meta.env.DEV && new URLSearchParams(location.search).has("e2e")) {
  const { install } = await import("./lib/e2e");
  install();
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
