import type {
  ViewerMergePreviewRequest,
  ViewerMergePreview,
  ViewerMergeRequest,
  ViewerMergeOutcome,
} from "@meowlive/contracts";
import {
  adminTransport,
  record,
  strings,
  integer,
  type ClientOptions,
} from "./companionship";
const summary = (v: unknown) =>
  record(v) &&
  typeof v.viewer_id === "string" &&
  (v.alias === null || typeof v.alias === "string") &&
  [
    "identities",
    "events",
    "memories",
    "relationships",
    "familiarity_milli",
    "affinity_milli",
  ].every((k) => integer(v[k]));
export function createViewerMergeClient(options: ClientOptions = {}) {
  const req = adminTransport(options);
  return {
    preview: (r: ViewerMergePreviewRequest, signal?: AbortSignal) =>
      req<ViewerMergePreview>(
        "/api/admin/viewers/merge/preview",
        (v) =>
          record(v) &&
          summary(v.source) &&
          summary(v.target) &&
          typeof v.fingerprint === "string" &&
          [
            "revision",
            "resulting_familiarity_milli",
            "resulting_affinity_milli",
          ].every((k) => integer(v[k])) &&
          Array.isArray(v.risks) &&
          v.risks.every((r) => typeof r === "string"),
        r,
        signal,
      ),
    apply: (r: ViewerMergeRequest, signal?: AbortSignal) =>
      req<ViewerMergeOutcome>(
        "/api/admin/viewers/merge/apply",
        (v) =>
          record(v) &&
          strings(v, ["canonical_viewer_id"]) &&
          integer(v.revision),
        r,
        signal,
      ),
  };
}
export type ViewerMergeClient = ReturnType<typeof createViewerMergeClient>;
