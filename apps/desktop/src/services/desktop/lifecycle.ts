/** Native exit notices stay within the services boundary. */
export async function listenForExitBlocked(onMessage: (message: string) => void): Promise<() => void> {
  if (!("__TAURI_INTERNALS__" in globalThis)) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<string>("desktop-exit-blocked", event => {
    if (typeof event.payload === "string" && event.payload.length <= 2000) onMessage(event.payload);
  });
}
