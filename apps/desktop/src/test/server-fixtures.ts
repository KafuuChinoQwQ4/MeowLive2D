import type { ServerStatus, SpeechSnapshot } from "@meowlive/contracts";
import { PROTOCOL_VERSION } from "@meowlive/contracts";

export function speech(overrides: Partial<SpeechSnapshot> = {}): SpeechSnapshot {
  return {
    id: "speech-1",
    generation: 0,
    text: "欢迎来到直播间",
    voice_id: "default",
    status: "queued",
    error: null,
    ...overrides,
  };
}

export function serverStatus(overrides: Partial<ServerStatus> = {}): ServerStatus {
  return {
    protocol_version: PROTOCOL_VERSION,
    session_id: "session-1",
    bridge_connected: true,
    generation: 0,
    speeches: [],
    ...overrides,
  };
}

export function jsonResponse(value: unknown, status = 200): Response {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  });
}

export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}
