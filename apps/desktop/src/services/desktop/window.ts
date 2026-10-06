/** Only the current native window is exposed to the application chrome. */
export const hasNativeWindow = () => "__TAURI_INTERNALS__" in globalThis;
export async function controlWindow(action: "minimize" | "toggleMaximize" | "close" | "startDragging") {
  if (!hasNativeWindow()) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow()[action]();
}
