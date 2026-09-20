import { readServerError, ServerRequestError } from "./responses";
import type {
  AdminSessionRequest,
  AdminSessionStatus,
  AdminSessionToken,
} from "@meowlive/contracts";

export type SessionStatus = AdminSessionStatus;
export type LoginResponse = AdminSessionToken;

export interface AdminSessionClient {
  readonly baseUrl: string;
  status(signal?: AbortSignal): Promise<SessionStatus>;
  login(token: string, signal?: AbortSignal): Promise<LoginResponse>;
  logout(signal?: AbortSignal): Promise<void>;
  subscribe(listener: (authenticated: boolean) => void): () => void;
}

type Fetcher = typeof fetch;

interface Session {
  token: string;
  epoch: number;
}

const sessions = new Map<string, Session>();
const listeners = new Map<string, Set<(authenticated: boolean) => void>>();
const operationEpochs = new Map<string, number>();
let nextSessionEpoch = 0;

function normalizeBaseUrl(value?: string): string {
  return (value ?? import.meta.env.VITE_MEOWLIVE_SERVER_URL ?? "http://127.0.0.1:19600").replace(/\/+$/, "");
}

function originOf(baseUrl: string): string {
  return new URL(baseUrl).origin;
}

function notify(origin: string, authenticated: boolean) {
  listeners.get(origin)?.forEach(listener => listener(authenticated));
}

function setSession(origin: string, token: string) {
  sessions.set(origin, { token, epoch: ++nextSessionEpoch });
  notify(origin, true);
}

function sameSession(left: Session | undefined, right: Session | undefined): boolean {
  return left !== undefined && right !== undefined && left.token === right.token && left.epoch === right.epoch;
}

function clearSession(origin: string, expected?: Session) {
  const current = sessions.get(origin);
  if (expected && !sameSession(current, expected)) return;
  if (sessions.delete(origin)) notify(origin, false);
}

function beginOperation(origin: string): number {
  const epoch = (operationEpochs.get(origin) ?? 0) + 1;
  operationEpochs.set(origin, epoch);
  return epoch;
}

function isCurrentOperation(origin: string, epoch: number): boolean {
  return operationEpochs.get(origin) === epoch;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readStatus(value: unknown): SessionStatus {
  if (!isRecord(value) || typeof value.enabled !== "boolean" || typeof value.authenticated !== "boolean") {
    throw new ServerRequestError("invalid_response", "主服务返回了无效的管理员会话状态。");
  }
  return { enabled: value.enabled, authenticated: value.authenticated };
}

function readLogin(value: unknown): LoginResponse {
  if (!isRecord(value)
    || typeof value.token !== "string"
    || value.token.trim().length === 0
    || value.token.length > 512
    || !Number.isInteger(value.expires_in_seconds)
    || (value.expires_in_seconds as number) < 1
    || (value.expires_in_seconds as number) > 86_400) {
    throw new ServerRequestError("invalid_response", "主服务返回了无效的管理员会话。");
  }
  return { token: value.token, expires_in_seconds: value.expires_in_seconds as number };
}

function requestError(value: unknown, status: number): ServerRequestError {
  const error = readServerError(value);
  return new ServerRequestError(
    error?.code ?? "http_error",
    error?.message ?? `主服务请求失败（HTTP ${status}）。`,
    status,
  );
}

export function createAuthenticatedFetch(baseUrl: string, fetcher: Fetcher = globalThis.fetch.bind(globalThis)): Fetcher {
  const origin = originOf(baseUrl);
  return async (input, init) => {
    const rawUrl = input instanceof Request ? input.url : input instanceof URL ? input.href : input;
    const url = new URL(rawUrl, baseUrl);
    const request = input instanceof Request ? new Request(input, init) : undefined;
    const headers = new Headers(request?.headers ?? init?.headers);
    const session = sessions.get(origin);
    const authenticatedApi = url.origin === origin && url.pathname.startsWith("/api/");
    const attachedSession = authenticatedApi && session && !headers.has("authorization") ? session : undefined;
    if (attachedSession) headers.set("authorization", `Bearer ${attachedSession.token}`);

    const response = request
      ? await fetcher(new Request(request, { headers, redirect: "error" }))
      : await fetcher(url.href, { ...init, headers, redirect: "error" });
    if (response.status === 401 && attachedSession) clearSession(origin, attachedSession);
    return response;
  };
}

export function createAdminSessionClient(options: { baseUrl?: string; fetcher?: Fetcher } = {}): AdminSessionClient {
  const baseUrl = normalizeBaseUrl(options.baseUrl);
  const origin = originOf(baseUrl);
  const rawFetch = options.fetcher ?? globalThis.fetch.bind(globalThis);
  const authenticatedFetch = createAuthenticatedFetch(baseUrl, rawFetch);

  async function read<T>(response: Response, parser: (value: unknown) => T): Promise<T> {
    const value: unknown = await response.json().catch(() => null);
    if (!response.ok) throw requestError(value, response.status);
    return parser(value);
  }

  return {
    baseUrl,
    async status(signal) {
      const expectedSession = sessions.get(origin);
      const result = await read(
        await authenticatedFetch(`${baseUrl}/api/admin/session`, { method: "GET", signal }),
        readStatus,
      );
      if (!result.authenticated && expectedSession) clearSession(origin, expectedSession);
      return result;
    },
    async login(token, signal) {
      const operation = beginOperation(origin);
      const result = await read(
        await rawFetch(`${baseUrl}/api/admin/session`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ token } satisfies AdminSessionRequest),
          redirect: "error",
          signal,
        }),
        readLogin,
      );
      if (isCurrentOperation(origin, operation)) setSession(origin, result.token);
      return result;
    },
    async logout(signal) {
      const operation = beginOperation(origin);
      const session = sessions.get(origin);
      clearSession(origin, session);
      try {
        await read(
          await rawFetch(`${baseUrl}/api/admin/session`, {
            method: "DELETE",
            ...(session ? { headers: { Authorization: `Bearer ${session.token}` } } : {}),
            redirect: "error",
            signal,
          }),
          () => undefined,
        );
      } finally {
        if (isCurrentOperation(origin, operation)) clearSession(origin);
      }
    },
    subscribe(listener) {
      const subscribers = listeners.get(origin) ?? new Set();
      subscribers.add(listener);
      listeners.set(origin, subscribers);
      return () => {
        subscribers.delete(listener);
        if (subscribers.size === 0) listeners.delete(origin);
      };
    },
  };
}
