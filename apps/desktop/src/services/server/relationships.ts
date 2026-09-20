import type {
  RelationshipPage,
  ViewerRelationship,
  RelationshipCreateRequest,
  RelationshipChangeRequest,
  KnowledgeMaintenanceRequest,
  GraphStatus,
} from "@meowlive/contracts";
import {
  adminTransport,
  record,
  strings,
  integer,
  viewerPath,
  type ClientOptions,
} from "./companionship";
const entity = (v: unknown) => record(v) && strings(v, ["kind", "id"]);
const fact = (v: unknown) =>
  record(v) &&
  strings(v, ["id", "kind", "confirmation"]) &&
  integer(v.version) &&
  entity(v.source) &&
  entity(v.target) &&
  typeof v.deleted === "boolean" &&
  (v.expires_at_ms === null || integer(v.expires_at_ms)) &&
  Array.isArray(v.evidence) &&
  v.evidence.every(
    (e) => record(e) && strings(e, ["source", "event_id", "quote"]),
  );
export function createRelationshipClient(options: ClientOptions = {}) {
  const req = adminTransport(options);
  return {
    list: (id: string, depth: number, signal?: AbortSignal) =>
      req<RelationshipPage>(
        `${viewerPath(id)}/relationships?depth=${depth}`,
        (v) =>
          record(v) &&
          typeof v.degraded === "boolean" &&
          Array.isArray(v.relationships) &&
          v.relationships.every(fact),
        undefined,
        signal,
      ),
    create: (r: RelationshipCreateRequest, signal?: AbortSignal) =>
      req<ViewerRelationship>("/api/admin/relationships", fact, r, signal),
    change: (id: string, r: RelationshipChangeRequest, signal?: AbortSignal) =>
      req<ViewerRelationship>(
        `/api/admin/relationships/${encodeURIComponent(id)}`,
        fact,
        r,
        signal,
      ),
    status: (signal?: AbortSignal) =>
      req<GraphStatus>(
        "/api/admin/graph/status",
        (v) =>
          record(v) &&
          ["pending", "leased", "failed"].every((k) => integer(v[k])) &&
          (v.oldest_pending_age_ms === null ||
            integer(v.oldest_pending_age_ms)) &&
          typeof v.connected === "boolean",
        undefined,
        signal,
      ),
    rebuild: (r: KnowledgeMaintenanceRequest, signal?: AbortSignal) =>
      req<{ queued: number }>(
        "/api/admin/graph/rebuild",
        (v) => record(v) && integer(v.queued),
        r,
        signal,
      ),
  };
}
export type RelationshipClient = ReturnType<typeof createRelationshipClient>;
