import type { ViewerEventPage, ViewerPage } from "@meowlive/contracts";
import { createAuthenticatedFetch } from "./auth";
import { readServerError, ServerRequestError } from "./responses";
import { isEventPayload } from "./eventPayload";

const PAGE_SIZE = 50;

export interface ViewerClient {
  readonly baseUrl: string;
  listViewers(offset: number, signal?: AbortSignal): Promise<ViewerPage>;
  listEvents(offset: number, signal?: AbortSignal): Promise<ViewerEventPage>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function validViewer(v: unknown): boolean {
  return isRecord(v) && typeof v.viewer_id === "string" && (v.current_alias === null || typeof v.current_alias === "string")
    && Array.isArray(v.aliases) && v.aliases.every(a => isRecord(a) && typeof a.alias === "string")
    && Array.isArray(v.identities) && v.identities.every(i => isRecord(i) && ["platform", "namespace", "id_kind", "external_id"].every(k => typeof i[k] === "string"));
}
function validEvent(e: unknown): boolean {
  return isRecord(e) && ["event_id", "source", "viewer"].every(k => typeof e[k] === "string") && isRecord(e.kind)
    && isEventPayload(e.kind)
    && (e.gift_metadata === undefined || e.gift_metadata === null || e.kind.type === "gift")
    && (e.gift_metadata === null || e.gift_metadata === undefined || (isRecord(e.gift_metadata) && (e.gift_metadata.price === undefined || e.gift_metadata.price === null || Number.isSafeInteger(e.gift_metadata.price))));
}

function isPage(value: unknown, events: boolean): boolean {
  return isRecord(value)
    && typeof value.scope_id === "string"
    && Number.isSafeInteger(value.offset)
    && (events ? Array.isArray(value.events) && value.events.every(validEvent) && Number.isSafeInteger(value.unconfirmed_events) : Array.isArray(value.viewers) && value.viewers.every(validViewer));
}

export function createViewerClient(options: { baseUrl?: string; fetcher?: typeof fetch } = {}): ViewerClient {
  const baseUrl = (options.baseUrl ?? import.meta.env.VITE_MEOWLIVE_SERVER_URL ?? "http://127.0.0.1:19600").replace(/\/+$/, "");
  const fetcher = options.fetcher ?? createAuthenticatedFetch(baseUrl);

  async function request<T>(path: string, signal: AbortSignal | undefined, valid: (value: unknown) => boolean): Promise<T> {
    const response = await fetcher(`${baseUrl}${path}`, { method: "GET", signal });
    const value: unknown = await response.json().catch(() => null);
    if (!response.ok) {
      const error = readServerError(value);
      throw new ServerRequestError(error?.code ?? "http_error", error?.message ?? `主服务请求失败（HTTP ${response.status}）。`, response.status);
    }
    if (!valid(value)) throw new ServerRequestError("invalid_response", "主服务返回了无效的观众记录。");
    return value as T;
  }

  const page = (offset: number) => {
    if (!Number.isSafeInteger(offset) || offset < 0 || offset > 1_000_000) throw new ServerRequestError("invalid_request", "分页位置无效。");
    return `?limit=${PAGE_SIZE}&offset=${offset}`;
  };

  return {
    baseUrl,
    listViewers: (offset, signal) => request<ViewerPage>(`/api/admin/viewers${page(offset)}`, signal, value => isPage(value, false)),
    listEvents: (offset, signal) => request<ViewerEventPage>(`/api/admin/events${page(offset)}`, signal, value => isPage(value, true)),
  };
}
