import type {
  CompanionshipDetail,
  CompanionshipHealth,
  AffinityAdjustmentRequest,
  AffinityReversalRequest,
  GiftConfirmationRequest,
  AdminMutationResult,
} from "@meowlive/contracts";
import { createAuthenticatedFetch } from "./auth";
import { readServerError, ServerRequestError } from "./responses";
export type ClientOptions = { baseUrl?: string; fetcher?: typeof fetch };
export const record = (v: unknown): v is Record<string, unknown> =>
  typeof v === "object" && v !== null && !Array.isArray(v);
export const integer = (v: unknown) =>
  typeof v === "number" && Number.isSafeInteger(v);
export const strings = (v: Record<string, unknown>, keys: string[]) =>
  keys.every((k) => typeof v[k] === "string");
export function adminTransport(options: ClientOptions = {}) {
  const baseUrl = (
    options.baseUrl ??
    import.meta.env.VITE_MEOWLIVE_SERVER_URL ??
    "http://127.0.0.1:19600"
  ).replace(/\/+$/, "");
  const fetcher = options.fetcher ?? createAuthenticatedFetch(baseUrl);
  return async <T>(
    path: string,
    valid: (v: unknown) => boolean,
    body?: unknown,
    signal?: AbortSignal,
  ): Promise<T> => {
    const r = await fetcher(`${baseUrl}${path}`, {
      method: body === undefined ? "GET" : "POST",
      signal,
      ...(body === undefined
        ? {}
        : {
            headers: { "content-type": "application/json" },
            body: JSON.stringify(body),
          }),
    });
    const value: unknown = await r.json().catch(() => null);
    if (!r.ok) {
      const e = readServerError(value);
      throw new ServerRequestError(
        e?.code ?? "http_error",
        e?.message ?? `管理请求失败（HTTP ${r.status}）。`,
        r.status,
      );
    }
    if (!valid(value))
      throw new ServerRequestError(
        "invalid_response",
        "管理服务返回无效数据，请刷新重试。",
      );
    return value as T;
  };
}
const nullableInteger = (v: unknown) => v === null || integer(v);
function validDetail(v: unknown): boolean {
  return (
    record(v) &&
    strings(v, ["viewer_id"]) &&
    [
      "familiarity_milli",
      "affinity_milli",
      "observed_days",
      "observed_sessions",
      "last_seen_at_ms",
    ].every((k) => integer(v[k])) &&
    nullableInteger(v.medal_level) &&
    nullableInteger(v.guard_level) &&
    Array.isArray(v.gifts) &&
    v.gifts.every(
      (g) =>
        record(g) &&
        strings(g, ["source", "event_id", "name", "value_kind"]) &&
        integer(g.count) &&
        integer(g.occurred_at_ms) &&
        nullableInteger(g.value_cents) &&
        (g.metadata === null ||
          (record(g.metadata) &&
            ["price", "medal_level", "guard_level"].every(
              (k) =>
                g.metadata &&
                record(g.metadata) &&
                (g.metadata[k] === undefined || nullableInteger(g.metadata[k])),
            ) &&
            (g.metadata.paid === undefined ||
              g.metadata.paid === null ||
              typeof g.metadata.paid === "boolean"))),
    ) &&
    Array.isArray(v.ledger) &&
    v.ledger.every(
      (l) =>
        record(l) &&
        strings(l, ["ledger_id", "kind", "reason", "actor"]) &&
        integer(l.created_at_ms) &&
        integer(l.computed_delta_milli) &&
        integer(l.applied_delta_milli) &&
        typeof l.reversible === "boolean" &&
        (l.reversed_ledger_id === null ||
          typeof l.reversed_ledger_id === "string"),
    )
  );
}
export const validMutation = (v: unknown) =>
  record(v) && typeof v.record_id === "string";
export const viewerPath = (id: string) =>
  `/api/admin/viewers/${encodeURIComponent(id)}`;
export function createCompanionshipClient(options: ClientOptions = {}) {
  const request = adminTransport(options);
  const mutate = (id: string, path: string, r: unknown, signal?: AbortSignal) =>
    request<AdminMutationResult>(
      `${viewerPath(id)}/${path}`,
      validMutation,
      r,
      signal,
    );
  return {
    detail: (id: string, signal?: AbortSignal) =>
      request<CompanionshipDetail>(
        viewerPath(id),
        validDetail,
        undefined,
        signal,
      ),
    status: (signal?: AbortSignal) =>
      request<CompanionshipHealth>(
        "/api/admin/companionship/status",
        (v) =>
          record(v) &&
          typeof v.durable_receipts === "boolean" &&
          Array.isArray(v.failed_receipts) &&
          v.failed_receipts.every(
            (r) =>
              record(r) &&
              typeof r.speech_id === "string" &&
              integer(r.attempts),
          ) &&
          integer(v.pending_receipts) &&
          integer(v.failed_receipt_attempts),
        undefined,
        signal,
      ),
    adjust: (id: string, r: AffinityAdjustmentRequest, signal?: AbortSignal) =>
      mutate(id, "adjust", r, signal),
    reverse: (id: string, r: AffinityReversalRequest, signal?: AbortSignal) =>
      mutate(id, "reverse", r, signal),
    confirmGift: (
      id: string,
      r: GiftConfirmationRequest,
      signal?: AbortSignal,
    ) => mutate(id, "gifts/confirm", r, signal),
  };
}
export type CompanionshipClient = ReturnType<typeof createCompanionshipClient>;
