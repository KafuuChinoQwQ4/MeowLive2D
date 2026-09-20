import type { LiveConnectionSnapshot, LiveSettingsSnapshot } from "@meowlive/contracts";
import { jsonResponse } from "./server-fixtures";

export function liveSnapshot(overrides: Partial<LiveConnectionSnapshot> = {}): LiveConnectionSnapshot {
  return {
    platform: "bilibili",
    configured: true,
    phase: "disconnected",
    room_id: null,
    accepted_events: 0,
    duplicate_events: 0,
    rejected_events: 0,
    reconnect_attempts: 0,
    last_error: null,
    ...overrides,
  };
}

export function liveSettingsSnapshot(overrides: Partial<LiveSettingsSnapshot> = {}): LiveSettingsSnapshot {
  return {
    enabled: true,
    app_id: "1234567890123456789",
    access_key_id_configured: true,
    access_key_secret_configured: true,
    identity_code_configured: true,
    storage_available: true,
    ...overrides,
  };
}

export function withLiveSettingsResponses(fetcher: typeof fetch): typeof fetch {
  return async (url, init) => String(url).endsWith("/api/live/settings")
    ? jsonResponse(liveSettingsSnapshot())
    : fetcher(url, init);
}
