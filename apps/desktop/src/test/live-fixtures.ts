import type { LiveConnectionSnapshot } from "@meowlive/contracts";

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
