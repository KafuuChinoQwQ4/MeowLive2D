import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { hasNativeWindow } from "./services/desktop/window";
import { shouldUseNativeGlass } from "./services/desktop/nativeAppearance";
import { App } from "./app/App";
import "./app/styles.css";
import "./app/workspace.css";

// Only Windows uses a transparent WebView so the native Acrylic backdrop can show.
document.documentElement.classList.toggle(
  "native-glass",
  shouldUseNativeGlass(hasNativeWindow(), navigator.userAgent),
);

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("Missing application root element.");
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
