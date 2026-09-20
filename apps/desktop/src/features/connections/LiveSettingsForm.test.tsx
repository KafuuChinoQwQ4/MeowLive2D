import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { LiveConnectionPhase, LiveSettingsRequest } from "@meowlive/contracts";
import { describe, expect, it, vi } from "vitest";
import { createLiveClient } from "../../services/server/live";
import { liveSettingsSnapshot, liveSnapshot } from "../../test/live-fixtures";
import { deferred, jsonResponse } from "../../test/server-fixtures";
import { ConnectionPanel } from "./ConnectionPanel";

function setup(options: { fresh?: boolean; phase?: LiveConnectionPhase; storage?: boolean; saveError?: boolean; readError?: boolean } = {}) {
  let saved = liveSettingsSnapshot(options.fresh ? {
    enabled: false, app_id: "", access_key_id_configured: false, access_key_secret_configured: false, identity_code_configured: false,
  } : {});
  saved.storage_available = options.storage ?? true;
  let readError = options.readError ?? false;
  let saveError = options.saveError ?? false;
  const writes: LiveSettingsRequest[] = [];
  const fetcher = vi.fn<typeof fetch>(async (url, init) => {
    const path = new URL(String(url)).pathname;
    if (path === "/api/live/settings" && init?.method === "POST") {
      const body = JSON.parse(String(init.body)) as LiveSettingsRequest;
      writes.push(body);
      if (saveError) return jsonResponse({ code: "save_failed", message: "保存失败，请重试" }, 500);
      saved = { ...saved, enabled: body.enabled, app_id: body.app_id,
        access_key_id_configured: !body.clear_credentials && (Boolean(body.access_key_id) || saved.access_key_id_configured),
        access_key_secret_configured: !body.clear_credentials && (Boolean(body.access_key_secret) || saved.access_key_secret_configured),
        identity_code_configured: !body.clear_credentials && (Boolean(body.identity_code) || saved.identity_code_configured),
      };
      return jsonResponse(saved);
    }
    if (path === "/api/live/settings") {
      if (readError) return jsonResponse({ code: "read_failed", message: "配置读取失败" }, 503);
      return jsonResponse(saved);
    }
    if (path === "/api/live") return jsonResponse(liveSnapshot({
      configured: saved.enabled && saved.access_key_id_configured && saved.access_key_secret_configured && saved.identity_code_configured,
      phase: options.phase ?? (saved.enabled ? "disconnected" : "disabled"),
    }));
    throw new Error(`Unexpected request ${path}`);
  });
  const client = createLiveClient({ fetcher });
  return { client, writes, fetcher, retryRead: () => { readError = false; }, retrySave: () => { saveError = false; } };
}

async function loaded() {
  await waitFor(() => expect(screen.getByLabelText("启用哔哩哔哩直播接入")).toBeEnabled());
}

function fillCredentials() {
  fireEvent.change(screen.getByLabelText("AccessKey ID"), { target: { value: " new-id " } });
  fireEvent.change(screen.getByLabelText("AccessKey Secret"), { target: { value: " new-secret " } });
  fireEvent.change(screen.getByLabelText("主播身份码"), { target: { value: " new-identity " } });
}

describe("哔哩哔哩直播配置", () => {
  it("首次填写保存后立即可连接，无须重启且不会自动连接", async () => {
    const server = setup({ fresh: true });
    render(<ConnectionPanel client={server.client} pollIntervalMs={60_000} />);
    await loaded();
    expect(screen.getByRole("button", { name: "连接直播间" })).toBeDisabled();
    fireEvent.click(screen.getByLabelText("启用哔哩哔哩直播接入"));
    fireEvent.change(screen.getByLabelText("应用 ID"), { target: { value: "9223372036854775807" } });
    fillCredentials();
    fireEvent.click(screen.getByRole("button", { name: "保存直播配置" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "连接直播间" })).toBeEnabled());
    expect(server.writes).toEqual([{ enabled: true, app_id: "9223372036854775807", access_key_id: "new-id", access_key_secret: "new-secret", identity_code: "new-identity", clear_credentials: false }]);
    expect(screen.getByLabelText("AccessKey Secret")).toHaveValue("");
    expect(screen.getByLabelText("主播身份码")).toHaveValue("");
    expect(screen.getByRole("heading", { name: "直播间未连接" })).toBeVisible();
    expect(screen.getByRole("link", { name: /哔哩哔哩直播开放平台/ })).toHaveAttribute("href", "https://open-live.bilibili.com/");
  });

  it("重新打开仅显示已保存状态，留空提交保留三项凭据", async () => {
    const server = setup();
    const view = render(<ConnectionPanel client={server.client} />);
    await loaded();
    expect(screen.getAllByText("已保存；留空继续使用")).toHaveLength(3);
    view.unmount();
    render(<ConnectionPanel client={server.client} />);
    await loaded();
    for (const label of ["AccessKey ID", "AccessKey Secret", "主播身份码"]) {
      expect(screen.getByLabelText(label)).toHaveValue("");
      expect(screen.getByLabelText(label)).toHaveAttribute("type", "password");
    }
    fireEvent.click(screen.getByRole("button", { name: "保存直播配置" }));
    await screen.findByText(/配置已保存并生效/);
    expect(server.writes[0]).toMatchObject({ access_key_id: null, access_key_secret: null, identity_code: null, clear_credentials: false });
  });

  it("保存失败保留填写内容，可直接重试", async () => {
    const server = setup({ saveError: true });
    render(<ConnectionPanel client={server.client} />);
    await loaded();
    fillCredentials();
    fireEvent.click(screen.getByRole("button", { name: "保存直播配置" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("保存失败，请重试");
    expect(screen.getByLabelText("AccessKey Secret")).toHaveValue(" new-secret ");
    expect(screen.queryByText(/配置已保存并生效/)).not.toBeInTheDocument();
    server.retrySave();
    fireEvent.click(screen.getByRole("button", { name: "保存直播配置" }));
    await screen.findByText(/配置已保存并生效/);
    expect(server.writes).toHaveLength(2);
  });

  it("配置加载失败时阻止保存，允许重新读取", async () => {
    const server = setup({ readError: true });
    render(<ConnectionPanel client={server.client} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("配置读取失败");
    expect(screen.getByRole("button", { name: "保存直播配置" })).toBeDisabled();
    server.retryRead();
    fireEvent.click(screen.getByRole("button", { name: "重新加载直播配置" }));
    await loaded();
    expect(screen.getByRole("button", { name: "保存直播配置" })).toBeEnabled();
  });

  it("加载完成前和没有本地存储时禁止写入", async () => {
    const read = deferred<Response>();
    const fetcher = vi.fn<typeof fetch>(async (url) => String(url).endsWith("/settings") ? read.promise : jsonResponse(liveSnapshot()));
    render(<ConnectionPanel client={createLiveClient({ fetcher })} />);
    expect(screen.getByRole("button", { name: "保存直播配置" })).toBeDisabled();
    read.resolve(jsonResponse(liveSettingsSnapshot({ storage_available: false })));
    expect(await screen.findByText(/无法保存直播配置/)).toBeVisible();
    expect(screen.getByRole("button", { name: "保存直播配置" })).toBeDisabled();
  });

  it.each(["connecting", "connected", "reconnecting", "disconnecting"] as const)("%s 时提示先断开并禁止保存", async (phase) => {
    const server = setup({ phase });
    render(<ConnectionPanel client={server.client} />);
    expect(await screen.findByText(/请先断开直播间/)).toBeVisible();
    expect(screen.getByRole("button", { name: "保存直播配置" })).toBeDisabled();
  });

  it.each(["0", "-1", "1.5", "1e3", "9223372036854775808"])("拒绝非法应用 ID %s 且不发送保存请求", async (appId) => {
    const server = setup();
    render(<ConnectionPanel client={server.client} />);
    await loaded();
    fireEvent.change(screen.getByLabelText("应用 ID"), { target: { value: appId } });
    fireEvent.click(screen.getByRole("button", { name: "保存直播配置" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("应用 ID");
    expect(server.writes).toHaveLength(0);
  });

  it("启用时凭据不齐全不提示成功，应用变更要求重新填写凭据", async () => {
    const server = setup();
    render(<ConnectionPanel client={server.client} />);
    await loaded();
    fireEvent.change(screen.getByLabelText("应用 ID"), { target: { value: "123" } });
    fireEvent.click(screen.getByRole("button", { name: "保存直播配置" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/重新填写/);
    expect(server.writes).toHaveLength(0);
    expect(screen.queryByText(/配置已保存并生效/)).not.toBeInTheDocument();
  });

  it("首次启用时阻止保存不完整凭据", async () => {
    const server = setup({ fresh: true });
    render(<ConnectionPanel client={server.client} />);
    await loaded();
    fireEvent.click(screen.getByLabelText("启用哔哩哔哩直播接入"));
    fireEvent.change(screen.getByLabelText("应用 ID"), { target: { value: "123" } });
    fireEvent.change(screen.getByLabelText("AccessKey ID"), { target: { value: "new-id" } });
    fireEvent.click(screen.getByRole("button", { name: "保存直播配置" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/AccessKey Secret.*主播身份码/);
    expect(server.writes).toHaveLength(0);
    expect(screen.queryByText(/配置已保存并生效/)).not.toBeInTheDocument();
  });

  it.each([
    ["AccessKey ID", "密".repeat(86)],
    ["AccessKey Secret", "密".repeat(171)],
    ["主播身份码", "密".repeat(171)],
    ["AccessKey Secret", "secret\u0007value"],
  ])("拒绝超长或含控制字符的 %s", async (label, value) => {
    const server = setup();
    render(<ConnectionPanel client={server.client} />);
    await loaded();
    fireEvent.change(screen.getByLabelText(label), { target: { value } });
    fireEvent.click(screen.getByRole("button", { name: "保存直播配置" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(label);
    expect(server.writes).toHaveLength(0);
    expect(screen.getByLabelText(label)).toHaveValue(value);
  });

  it("保存期间阻止重复提交和连接，卸载时取消保存请求", async () => {
    const pending = deferred<Response>();
    let saveSignal: AbortSignal | null | undefined;
    const writes: unknown[] = [];
    const fetcher = vi.fn<typeof fetch>(async (url, init) => {
      if (init?.method === "POST") { saveSignal = init.signal; writes.push(init.body); return pending.promise; }
      return jsonResponse(String(url).endsWith("/settings") ? liveSettingsSnapshot() : liveSnapshot());
    });
    const view = render(<ConnectionPanel client={createLiveClient({ fetcher })} />);
    await loaded();
    fireEvent.click(screen.getByRole("button", { name: "保存直播配置" }));
    const saving = screen.getByRole("button", { name: "正在保存直播配置…" });
    expect(saving).toBeDisabled();
    expect(screen.getByRole("button", { name: "连接直播间" })).toBeDisabled();
    fireEvent.click(saving);
    expect(writes).toHaveLength(1);
    view.unmount();
    expect(saveSignal?.aborted).toBe(true);
  });

  it("保存时切换主服务会取消旧请求，新服务配置加载后仍可编辑", async () => {
    const pending = deferred<Response>();
    let oldSignal: AbortSignal | null | undefined;
    const firstClient = createLiveClient({ fetcher: async (url, init) => {
      if (init?.method === "POST") { oldSignal = init.signal; return pending.promise; }
      return jsonResponse(String(url).endsWith("/settings") ? liveSettingsSnapshot() : liveSnapshot());
    } });
    const view = render(<ConnectionPanel client={firstClient} />);
    await loaded();
    fireEvent.click(screen.getByRole("button", { name: "保存直播配置" }));
    const second = setup();
    view.rerender(<ConnectionPanel client={second.client} />);
    await loaded();
    expect(oldSignal?.aborted).toBe(true);
    expect(screen.getByRole("button", { name: "保存直播配置" })).toBeEnabled();
  });

  it("保存后状态刷新失败时禁用连接，避免继续使用保存前的状态", async () => {
    let saved = false;
    const fetcher: typeof fetch = async (url, init) => {
      if (String(url).endsWith("/settings")) {
        if (init?.method === "POST") saved = true;
        return jsonResponse(liveSettingsSnapshot({ enabled: !saved }));
      }
      return saved ? jsonResponse({ code: "unavailable", message: "直播状态暂时不可读" }, 503) : jsonResponse(liveSnapshot());
    };
    render(<ConnectionPanel client={createLiveClient({ fetcher })} />);
    await loaded();
    fireEvent.click(screen.getByLabelText("启用哔哩哔哩直播接入"));
    fireEvent.click(screen.getByRole("button", { name: "保存直播配置" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("直播状态暂时不可读");
    expect(screen.getByRole("button", { name: "连接直播间" })).toBeDisabled();
  });

  it("显式移除凭据会关闭接入，保存前告知影响", async () => {
    const server = setup();
    render(<ConnectionPanel client={server.client} />);
    await loaded();
    fireEvent.click(screen.getByLabelText("移除已保存的全部直播凭据"));
    expect(screen.getByLabelText("启用哔哩哔哩直播接入")).not.toBeChecked();
    expect(screen.getByText(/保存后会移除/)).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "保存直播配置" }));
    await screen.findByText(/配置已保存并生效/);
    expect(server.writes[0]).toMatchObject({ enabled: false, clear_credentials: true, access_key_id: null, access_key_secret: null, identity_code: null });
    expect(screen.getByRole("button", { name: "连接直播间" })).toBeDisabled();
  });
});
