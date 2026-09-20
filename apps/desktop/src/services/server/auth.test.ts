import { describe, expect, it, vi } from "vitest";
import { createAdminSessionClient, createAuthenticatedFetch } from "./auth";
import { jsonResponse } from "../../test/server-fixtures";

describe("管理员会话服务", () => {
  it("只向同一主服务 origin 的 API 请求附加内存中的 Bearer 凭据", async () => {
    const fetcher = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(jsonResponse({ token: "session-token", expires_in_seconds: 900 }))
      .mockResolvedValue(jsonResponse({ enabled: true, authenticated: false }));
    const auth = createAdminSessionClient({ baseUrl: "http://127.0.0.1:19991", fetcher });
    await auth.login("initial-token");
    const request = createAuthenticatedFetch("http://127.0.0.1:19991", fetcher);

    await request("http://127.0.0.1:19991/api/agent", { headers: { "Content-Type": "application/json" } });
    await request("https://example.invalid/api/agent");
    await request("http://127.0.0.1:19991/not-api");

    expect(fetcher.mock.calls[1]).toEqual([
      "http://127.0.0.1:19991/api/agent",
      expect.objectContaining({ redirect: "error", headers: expect.any(Headers) }),
    ]);
    const sameOrigin = fetcher.mock.calls[1][1]?.headers as Headers;
    expect(sameOrigin.get("authorization")).toBe("Bearer session-token");
    expect(sameOrigin.get("content-type")).toBe("application/json");
    expect((fetcher.mock.calls[2][1]?.headers as Headers).get("authorization")).toBeNull();
    expect((fetcher.mock.calls[3][1]?.headers as Headers).get("authorization")).toBeNull();
  });

  it("保留 Request 的方法、请求体和取消信号", async () => {
    const fetcher = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(jsonResponse({ token: "session-token", expires_in_seconds: 900 }))
      .mockResolvedValue(jsonResponse({ ok: true }));
    const auth = createAdminSessionClient({ baseUrl: "http://127.0.0.1:19994", fetcher });
    await auth.login("initial-token");
    const controller = new AbortController();
    const input = new Request("http://127.0.0.1:19994/api/agent", {
      method: "POST",
      body: "event-body",
      headers: { "X-Request-Id": "request-1" },
      signal: controller.signal,
    });

    await createAuthenticatedFetch("http://127.0.0.1:19994", fetcher)(input);

    const forwarded = fetcher.mock.calls[1][0] as Request;
    expect(forwarded.method).toBe("POST");
    expect(forwarded.headers.get("x-request-id")).toBe("request-1");
    expect(forwarded.headers.get("authorization")).toBe("Bearer session-token");
    expect(await forwarded.text()).toBe("event-body");
    controller.abort();
    expect(forwarded.signal.aborted).toBe(true);
  });

  it("clears only the matching origin token after a 401 response", async () => {
    const fetcher = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(jsonResponse({ token: "session-token", expires_in_seconds: 900 }))
      .mockResolvedValueOnce(new Response(null, { status: 401 }))
      .mockResolvedValueOnce(jsonResponse({ enabled: true, authenticated: false }));
    const auth = createAdminSessionClient({ baseUrl: "http://127.0.0.1:19992", fetcher });
    await auth.login("initial-token");

    await createAuthenticatedFetch("http://127.0.0.1:19992", fetcher)("/api/agent");
    await auth.status();

    expect((fetcher.mock.calls[2][1]?.headers as Headers).get("authorization")).toBeNull();
  });

  it("不会因调用方显式 Authorization 的 401 清除内存会话", async () => {
    const fetcher = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(jsonResponse({ token: "session-token", expires_in_seconds: 900 }))
      .mockResolvedValueOnce(new Response(null, { status: 401 }))
      .mockResolvedValueOnce(jsonResponse({ ok: true }));
    const auth = createAdminSessionClient({ baseUrl: "http://127.0.0.1:19993", fetcher });
    await auth.login("initial-token");
    const request = createAuthenticatedFetch("http://127.0.0.1:19993", fetcher);

    await request("/api/agent", { headers: { Authorization: "Bearer caller-token" } });
    await request("/api/agent");

    expect((fetcher.mock.calls[1][1]?.headers as Headers).get("authorization")).toBe("Bearer caller-token");
    expect((fetcher.mock.calls[2][1]?.headers as Headers).get("authorization")).toBe("Bearer session-token");
  });

  it("不会让较早的未认证状态响应清除之后登录的令牌", async () => {
    let resolveStatus: ((response: Response) => void) | undefined;
    const fetcher = vi.fn<typeof fetch>()
      .mockImplementationOnce(() => new Promise<Response>(resolve => { resolveStatus = resolve; }))
      .mockResolvedValueOnce(jsonResponse({ token: "new-session", expires_in_seconds: 900 }))
      .mockResolvedValueOnce(jsonResponse({ ok: true }));
    const auth = createAdminSessionClient({ baseUrl: "http://127.0.0.1:19995", fetcher });

    const status = auth.status();
    await auth.login("initial-token");
    resolveStatus?.(jsonResponse({ enabled: true, authenticated: false }));
    await status;
    await createAuthenticatedFetch("http://127.0.0.1:19995", fetcher)("/api/agent");

    expect((fetcher.mock.calls[2][1]?.headers as Headers).get("authorization")).toBe("Bearer new-session");
  });

  it("不会让迟到的退出清除之后登录的令牌", async () => {
    let resolveLogout: ((response: Response) => void) | undefined;
    const fetcher = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(jsonResponse({ token: "initial-session", expires_in_seconds: 900 }))
      .mockImplementationOnce(() => new Promise<Response>(resolve => { resolveLogout = resolve; }))
      .mockResolvedValueOnce(jsonResponse({ token: "new-session", expires_in_seconds: 900 }))
      .mockResolvedValueOnce(jsonResponse({ ok: true }));
    const auth = createAdminSessionClient({ baseUrl: "http://127.0.0.1:19996", fetcher });
    await auth.login("initial-token");

    const logout = auth.logout();
    await auth.login("new-token");
    resolveLogout?.(jsonResponse({}));
    await logout;
    await createAuthenticatedFetch("http://127.0.0.1:19996", fetcher)("/api/agent");

    expect((fetcher.mock.calls[3][1]?.headers as Headers).get("authorization")).toBe("Bearer new-session");
  });

  it("不会让迟到的登录在退出后恢复会话", async () => {
    let resolveLogin: ((response: Response) => void) | undefined;
    const fetcher = vi.fn<typeof fetch>()
      .mockImplementationOnce(() => new Promise<Response>(resolve => { resolveLogin = resolve; }))
      .mockResolvedValueOnce(jsonResponse({}))
      .mockResolvedValueOnce(jsonResponse({ ok: true }));
    const auth = createAdminSessionClient({ baseUrl: "http://127.0.0.1:19997", fetcher });

    const login = auth.login("initial-token");
    await auth.logout();
    resolveLogin?.(jsonResponse({ token: "late-session", expires_in_seconds: 900 }));
    await login;
    await createAuthenticatedFetch("http://127.0.0.1:19997", fetcher)("/api/agent");

    expect((fetcher.mock.calls[2][1]?.headers as Headers).get("authorization")).toBeNull();
  });
});
