import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { AdminGate } from "./AdminGate";
import { createAdminSessionClient, type AdminSessionClient, type SessionStatus } from "../services/server/auth";
import { jsonResponse } from "../test/server-fixtures";

function client(overrides: Partial<AdminSessionClient> = {}): AdminSessionClient {
  const listeners = new Set<(authenticated: boolean) => void>();
  return {
    baseUrl: "http://127.0.0.1:19993",
    status: vi.fn().mockResolvedValue({ enabled: true, authenticated: false }),
    login: vi.fn().mockImplementation(async () => {
      listeners.forEach(listener => listener(true));
      return { token: "session-token", expires_in_seconds: 900 };
    }),
    logout: vi.fn().mockImplementation(async () => {
      listeners.forEach(listener => listener(false));
    }),
    subscribe: vi.fn(listener => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    }),
    ...overrides,
  };
}

it("默认控制面板用户直接管理，不显示管理员登录或退出", async () => {
  const api = client({ status: vi.fn().mockResolvedValue({ enabled: false, authenticated: true }) });
  render(<AdminGate client={api}><p>观众与事件</p></AdminGate>);
  expect(await screen.findByText("观众与事件")).toBeInTheDocument();
  expect(screen.queryByLabelText("管理员口令")).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /管理员/ })).not.toBeInTheDocument();
  expect(api.login).not.toHaveBeenCalled();
});

it("does not mount protected content before a successful administrator login", async () => {
  const api = client();
  const mounted = vi.fn();
  function Protected() {
    mounted();
    return <p>受保护内容</p>;
  }
  render(<AdminGate client={api}><Protected /></AdminGate>);

  expect(await screen.findByRole("button", { name: "登录管理员" })).toBeDisabled();
  expect(mounted).not.toHaveBeenCalled();
  await userEvent.type(screen.getByLabelText("管理员口令"), "initial-token");
  await userEvent.click(screen.getByRole("button", { name: "登录管理员" }));

  expect(api.login).toHaveBeenCalledWith("initial-token");
  expect(await screen.findByText("受保护内容")).toBeInTheDocument();
  expect(screen.queryByLabelText("管理员口令")).not.toBeInTheDocument();
});

it("真实会话通知不会让成功登录按钮停留在忙碌状态", async () => {
  const fetcher = vi.fn<typeof fetch>()
    .mockResolvedValueOnce(jsonResponse({ enabled: true, authenticated: false }))
    .mockResolvedValueOnce(jsonResponse({ token: "session-token", expires_in_seconds: 900 }));
  const api = createAdminSessionClient({ baseUrl: "http://127.0.0.1:19998", fetcher });
  render(<AdminGate client={api}><p>受保护内容</p></AdminGate>);

  await userEvent.type(await screen.findByLabelText("管理员口令"), "initial-token");
  await userEvent.click(screen.getByRole("button", { name: "登录管理员" }));

  expect(await screen.findByText("受保护内容")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "退出管理员" })).toBeEnabled();
});

it("unmounts protected content and returns to login after logout", async () => {
  const api = client({ status: vi.fn().mockResolvedValue({ enabled: true, authenticated: true }) });
  render(<AdminGate client={api}><p>受保护内容</p></AdminGate>);

  expect(await screen.findByText("受保护内容")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "退出管理员" }));

  expect(api.logout).toHaveBeenCalledOnce();
  expect(await screen.findByRole("button", { name: "登录管理员" })).toBeDisabled();
  expect(screen.queryByText("受保护内容")).not.toBeInTheDocument();
});

it("状态不可用时允许重新核对", async () => {
  const api = client({
    status: vi.fn()
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValueOnce({ enabled: false, authenticated: false }),
  });
  render(<AdminGate client={api}><p>受保护内容</p></AdminGate>);

  expect(await screen.findByRole("alert")).toHaveTextContent("无法连接控制面板服务");
  await userEvent.click(screen.getByRole("button", { name: "重新核对" }));

  expect(await screen.findByText("受保护内容")).toBeInTheDocument();
  expect(api.status).toHaveBeenCalledTimes(2);
});

it("会话通知会使在途状态响应失效", async () => {
  let resolveStatus: ((status: SessionStatus) => void) | undefined;
  let notify: ((authenticated: boolean) => void) | undefined;
  const api = client({
    status: vi.fn<AdminSessionClient["status"]>(() => new Promise<SessionStatus>(resolve => { resolveStatus = resolve; })),
    subscribe: vi.fn(listener => {
      notify = listener;
      return () => {};
    }),
  });
  render(<AdminGate client={api}><p>受保护内容</p></AdminGate>);

  await act(async () => { notify?.(true); });
  expect(screen.getByText("受保护内容")).toBeInTheDocument();
  await act(async () => { resolveStatus?.({ enabled: true, authenticated: false }); });

  expect(screen.getByText("受保护内容")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "登录管理员" })).not.toBeInTheDocument();
});

it("其他页面的会话通知会使本页在途登录失效", async () => {
  let resolveLogin: ((value?: unknown) => void) | undefined;
  let notify: ((authenticated: boolean) => void) | undefined;
  const api = client({
    login: vi.fn(() => new Promise(resolve => { resolveLogin = resolve; }).then(() => ({ token: "session-token", expires_in_seconds: 900 }))),
    subscribe: vi.fn(listener => {
      notify = listener;
      return () => {};
    }),
  });
  render(<AdminGate client={api}><p>受保护内容</p></AdminGate>);

  await userEvent.type(await screen.findByLabelText("管理员口令"), "initial-token");
  await userEvent.click(screen.getByRole("button", { name: "登录管理员" }));
  await act(async () => { notify?.(false); });
  resolveLogin?.();
  await act(async () => {});

  expect(screen.queryByText("受保护内容")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "登录管理员" })).toBeDisabled();
});
