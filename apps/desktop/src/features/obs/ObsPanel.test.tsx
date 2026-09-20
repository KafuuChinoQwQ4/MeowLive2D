import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it } from "vitest";
import { ObsPanel } from "./ObsPanel";
import { createObsClient } from "../../services/server/obs";

const settings = { enabled: true, websocket_url: "ws://127.0.0.1:4455", password_configured: true, storage_available: true };

it("only reads status on mount and refreshes scene and recording state after explicit actions", async () => {
  const operations: string[] = [];
  let scene = "主场景", recording = false;
  const client = createObsClient({ fetcher: async (_url, init) => {
    if (String(_url).endsWith("/settings")) return Response.json(settings);
    const op = init?.method === "POST" ? JSON.parse(String(init.body)) : { type: "status" };
    operations.push(op.type);
    if (op.type === "start_recording") recording = true;
    if (op.type === "stop_recording") recording = false;
    if (op.type === "set_scene") scene = op.scene_name;
    return Response.json({ connected: true, recording, current_scene: scene, scenes: ["主场景", "休息"] });
  } });
  render(<ObsPanel client={client} />);
  const start = await screen.findByRole("button", { name: "开始录制" });
  expect(operations).toEqual(["status"]);
  const user = userEvent.setup();
  await user.click(start);
  expect(await screen.findByText("正在录制")).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "停止录制" }));
  await screen.findByText("未录制");
  await user.selectOptions(screen.getByRole("combobox", { name: "OBS 场景" }), "休息");
  await user.click(screen.getByRole("button", { name: "切换场景" }));
  expect(await screen.findByText("当前场景：休息")).toBeInTheDocument();
  expect(operations).toEqual(["status", "start_recording", "stop_recording", "set_scene"]);
});

it("marks failed operations unknown and requires a refresh before another mutation", async () => {
  let count = 0;
  const client = createObsClient({ fetcher: async (url) => {
    if (String(url).endsWith("/settings")) return Response.json(settings);
    if (++count === 1) return Response.json({ connected: true, recording: false, current_scene: "主场景", scenes: ["主场景"] });
    throw new Error("offline");
  } });
  render(<ObsPanel client={client} />);
  await userEvent.click(await screen.findByRole("button", { name: "开始录制" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("请刷新状态确认");
  expect(screen.getByRole("button", { name: "开始录制" })).toBeDisabled();
});

it("saves an edited address and password locally without starting recording, then tests only on request", async () => {
  const writes: unknown[] = [];
  let statusReads = 0;
  let stored = { ...settings, enabled: false, password_configured: false };
  const client = createObsClient({ fetcher: async (url, init) => {
    if (String(url).endsWith("/settings")) {
      if (init?.method === "POST") {
        const request = JSON.parse(String(init.body));
        writes.push(request);
        stored = { ...stored, enabled: request.enabled, websocket_url: request.websocket_url, password_configured: !!request.password };
      }
      return Response.json(stored);
    }
    statusReads++;
    expect(init?.method).toBe("GET");
    return Response.json({ connected: stored.enabled, recording: false, current_scene: stored.enabled ? "主场景" : "", scenes: stored.enabled ? ["主场景"] : [] });
  } });
  render(<ObsPanel client={client} />);
  const user = userEvent.setup();
  const address = await screen.findByRole("textbox", { name: "OBS WebSocket 地址" });
  await user.click(screen.getByRole("checkbox", { name: "启用 OBS 控制" }));
  await user.clear(address);
  await user.type(address, "ws://127.0.0.1:4456");
  await user.type(screen.getByLabelText("OBS WebSocket 密码"), "private-password");
  expect(screen.getByText("有未保存的 OBS 设置，请先保存再测试连接。")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "刷新 OBS 状态" })).toBeDisabled();
  await user.click(screen.getByRole("button", { name: "保存 OBS 设置" }));
  expect(writes).toEqual([{ enabled: true, websocket_url: "ws://127.0.0.1:4456", password: "private-password", clear_password: false }]);
  expect(screen.getByLabelText("OBS WebSocket 密码")).toHaveValue("");
  expect(statusReads).toBe(1);
  await user.click(screen.getByRole("button", { name: "测试 OBS 连接" }));
  expect(await screen.findByText("当前场景：主场景")).toBeInTheDocument();
  expect(statusReads).toBe(2);
});

it("preserves the form on failed saves and supports explicit password removal", async () => {
  let saveFails = true;
  const requests: unknown[] = [];
  const client = createObsClient({ fetcher: async (url, init) => {
    if (String(url).endsWith("/settings")) {
      if (init?.method === "POST") {
        requests.push(JSON.parse(String(init.body)));
        if (saveFails) throw new Error("offline");
        return Response.json({ ...settings, password_configured: false });
      }
      return Response.json(settings);
    }
    return Response.json({ connected: false, recording: false, current_scene: "", scenes: [] });
  } });
  render(<ObsPanel client={client} />);
  const user = userEvent.setup();
  await screen.findByText("已设置密码；留空保留现有密码。");
  await user.click(screen.getByRole("checkbox", { name: "清除已保存密码" }));
  await user.click(screen.getByRole("button", { name: "保存 OBS 设置" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("无法连接");
  expect(screen.getByRole("checkbox", { name: "清除已保存密码" })).toBeChecked();
  saveFails = false;
  await user.click(screen.getByRole("button", { name: "保存 OBS 设置" }));
  expect(await screen.findByText("尚未设置密码。")).toBeInTheDocument();
  expect(requests).toEqual(Array(2).fill({ enabled: true, websocket_url: "ws://127.0.0.1:4455", password: null, clear_password: true }));
});

it("invalidates the previous OBS connection state when settings are reloaded", async () => {
  let reads = 0;
  const client = createObsClient({ fetcher: async (url) => {
    if (String(url).endsWith("/settings")) return Response.json({ ...settings, websocket_url: ++reads === 1 ? settings.websocket_url : "ws://127.0.0.1:4456" });
    return Response.json({ connected: true, recording: false, current_scene: "主场景", scenes: ["主场景"] });
  } });
  render(<ObsPanel client={client} />);
  await screen.findByText("当前场景：主场景");
  await userEvent.click(screen.getByRole("button", { name: "重新读取 OBS 设置" }));
  expect(screen.getByRole("textbox", { name: "OBS WebSocket 地址" })).toHaveValue("ws://127.0.0.1:4456");
  expect(screen.getByRole("button", { name: "开始录制" })).toBeDisabled();
  expect(screen.queryByText("当前场景：主场景")).not.toBeInTheDocument();
});
