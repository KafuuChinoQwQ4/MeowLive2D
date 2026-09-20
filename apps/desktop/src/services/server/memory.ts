import type {
  KnowledgeMaintenanceRequest,
  MemoryPage,
  MemoryMutationRequest,
  MemoryJobsStatus,
} from "@meowlive/contracts";
import {
  adminTransport,
  record,
  strings,
  integer,
  viewerPath,
  type ClientOptions,
} from "./companionship";
export function createMemoryClient(options: ClientOptions = {}) {
  const request = adminTransport(options);
  return {
    retry: (r: KnowledgeMaintenanceRequest, signal?: AbortSignal) =>
      request<{ updated: true }>(
        "/api/admin/memories/retry",
        (v) => record(v) && v.updated === true,
        r,
        signal,
      ),
    rebuildVectors: (r: KnowledgeMaintenanceRequest, signal?: AbortSignal) =>
      request<{ updated: true }>(
        "/api/admin/memories/rebuild-vectors",
        (v) => record(v) && v.updated === true,
        r,
        signal,
      ),
    list: (id: string, signal?: AbortSignal) =>
      request<MemoryPage>(
        `${viewerPath(id)}/memories`,
        (v) =>
          record(v) &&
          Array.isArray(v.memories) &&
          v.memories.every(
            (m) =>
              record(m) &&
              strings(m, [
                "id",
                "viewer_id",
                "key",
                "value",
                "kind",
                "status",
              ]) &&
              integer(m.version) &&
              typeof m.locked === "boolean" &&
              typeof m.deleted === "boolean" &&
              (m.expires_at_ms === null || integer(m.expires_at_ms)) &&
              Array.isArray(m.evidence) &&
              m.evidence.every(
                (e) =>
                  record(e) &&
                  strings(e, ["source", "event_id", "quote"]) &&
                  integer(e.occurred_at_ms),
              ),
          ),
        undefined,
        signal,
      ),
    status: (signal?: AbortSignal) =>
      request<MemoryJobsStatus>(
        "/api/admin/memories/status",
        (v) =>
          record(v) &&
          [
            "pending",
            "running",
            "failed",
            "embedding_pending",
            "embedding_failed",
          ].every((k) => integer(v[k])),
        undefined,
        signal,
      ),
    mutate: (
      id: string,
      memory: string,
      r: MemoryMutationRequest,
      signal?: AbortSignal,
    ) =>
      request<{ updated: true }>(
        `${viewerPath(id)}/memories/${encodeURIComponent(memory)}`,
        (v) => record(v) && v.updated === true,
        r,
        signal,
      ),
  };
}
export type MemoryClient = ReturnType<typeof createMemoryClient>;
