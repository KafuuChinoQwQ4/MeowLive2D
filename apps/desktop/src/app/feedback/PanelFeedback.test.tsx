import { act, fireEvent, render, renderHook, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { FeedbackProvider } from "./OperationFeedback";
import { useConnectionController } from "../../features/connections/useConnectionController";
import { useSpeechController } from "../../features/live/useSpeechController";
import { useAgentController } from "../../features/agent/useAgentController";
import { useModelLibrary } from "../../features/model-library/useModelLibrary";
import { useResourcesController } from "../resources/useResourcesController";
import { EventSimulator } from "../../features/agent/EventSimulator";
import { ObsPanel } from "../../features/obs/ObsPanel";
import { LlmPanel } from "../../features/llm/LlmPanel";
import { VoicePanel } from "../../features/voices/VoicePanel";
import { CharacterPanel } from "../../features/characters/CharacterPanel";
import type { VoiceController } from "../../features/voices/types";
import type { CharacterController } from "../../features/characters/types";
import { liveSnapshot } from "../../test/live-fixtures";
import { serverStatus, speech } from "../../test/server-fixtures";
import { agentEvent, agentStatus } from "../../test/agent-fixtures";
import { modelLibrarySnapshot } from "../../test/model-library-fixtures";
import { resourceSnapshot } from "../../test/resource-fixtures";
import type { LiveClient } from "../../services/server/live";
import type { ServerClient } from "../../services/server";
import type { AgentClient } from "../../services/server/agent";
import type { ModelLibraryClient } from "../../services/model-library";
import type { ResourcesClient } from "../../services/server/resources";
import type { LlmClient } from "../../services/server/llm";

it("reports a failed connection snapshot as failure even when the request resolves", async () => {
  const client: LiveClient = { baseUrl: "test", getStatus: vi.fn().mockResolvedValue(liveSnapshot()), connect: vi.fn().mockResolvedValue(liveSnapshot({ phase: "failed", last_error: "平台拒绝连接" })), disconnect: vi.fn() };
  const { result } = renderHook(() => useConnectionController(client, 1000), { wrapper: FeedbackProvider });
  await act(async () => {});
  await act(async () => { await result.current.connect(); });
  expect(screen.getByRole("dialog")).toHaveTextContent("平台拒绝连接");
  expect(screen.getByRole("dialog")).toHaveClass("is-error");
});

it("shows a completed live connection once after the accepted connection request", async () => {
  vi.useFakeTimers(); let snapshot = liveSnapshot();
  const client: LiveClient = { baseUrl: "test", getStatus: vi.fn(async () => snapshot), connect: vi.fn().mockResolvedValue(liveSnapshot({ phase: "connecting" })), disconnect: vi.fn() };
  const { result } = renderHook(() => useConnectionController(client, 1000), { wrapper: FeedbackProvider });
  await act(async () => {});
  await act(async () => { await result.current.connect(); });
  expect(screen.getByRole("dialog")).toHaveTextContent("请求已提交");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  snapshot = liveSnapshot({ phase: "connected" });
  await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  expect(screen.getByRole("dialog")).toHaveTextContent("直播间已连接");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

it("acknowledges queued speech then reports the later playback failure once", async () => {
  vi.useFakeTimers(); let status = serverStatus();
  const client: ServerClient = { baseUrl: "test", getStatus: vi.fn(async () => status), submitSpeech: vi.fn().mockResolvedValue(speech()), stop: vi.fn() };
  const { result } = renderHook(() => useSpeechController(client, 1000), { wrapper: FeedbackProvider });
  await act(async () => {});
  await act(async () => { await result.current.submit({ text: "你好", voice_id: "active" }); });
  expect(screen.getByRole("dialog")).toHaveTextContent("播报已提交");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  status = serverStatus({ speeches: [speech({ status: "failed", error: "音频设备不可用" })] });
  await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  expect(screen.getByRole("dialog")).toHaveTextContent("音频设备不可用");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

it("reports Agent pause success and an explicit resume failure", async () => {
  const client: AgentClient = { baseUrl: "test", getStatus: vi.fn().mockResolvedValue(agentStatus()), pause: vi.fn().mockResolvedValue(agentStatus()), resume: vi.fn().mockRejectedValue(new Error("LLM 未配置")), saveSettings: vi.fn(), submitEvents: vi.fn() };
  const { result } = renderHook(() => useAgentController(client, 1000), { wrapper: FeedbackProvider });
  await act(async () => {});
  await act(async () => { await result.current.pause(); });
  expect(screen.getByRole("dialog")).toHaveTextContent("Agent 已暂停");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  await act(async () => { await result.current.resume(); });
  expect(screen.getByRole("dialog")).toHaveTextContent("LLM 未配置");
});

it("does not claim Agent resumed when the returned state is still paused", async () => {
  const client: AgentClient = { baseUrl: "test", getStatus: vi.fn().mockResolvedValue(agentStatus()), pause: vi.fn(), resume: vi.fn().mockResolvedValue(agentStatus()), saveSettings: vi.fn(), submitEvents: vi.fn() };
  const { result } = renderHook(() => useAgentController(client, 1000), { wrapper: FeedbackProvider });
  await act(async () => {});
  await act(async () => { await result.current.resume(); });
  expect(screen.getByRole("dialog")).toHaveClass("is-error");
});

it("reports a new Agent event failure once without reopening historical failures", async () => {
  vi.useFakeTimers();
  let snapshot = agentStatus({ events: [agentEvent()] });
  const client: AgentClient = { baseUrl: "test", getStatus: vi.fn(async () => snapshot), pause: vi.fn(), resume: vi.fn(), saveSettings: vi.fn(), submitEvents: vi.fn() };
  renderHook(() => useAgentController(client, 1000), { wrapper: FeedbackProvider });
  await act(async () => {});
  snapshot = agentStatus({ events: [agentEvent({ status: "failed", error: "语音引擎不可用" })] });
  await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  expect(screen.getByRole("dialog")).toHaveTextContent("语音引擎不可用");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  await act(async () => { await vi.advanceTimersByTimeAsync(2000); });
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

it("reports replay validation failure before submitting events", async () => {
  render(<FeedbackProvider><EventSimulator disabled={false} onSubmit={vi.fn()} /></FeedbackProvider>);
  fireEvent.change(screen.getByLabelText("事件回放 JSON"), { target: { value: "not json" } });
  fireEvent.click(screen.getByRole("button", { name: "提交事件回放" }));
  expect(screen.getByRole("dialog")).toHaveTextContent("回放内容不是有效的 JSON");
});

it("acknowledges downloading separately from model availability", async () => {
  const snapshot = modelLibrarySnapshot();
  const client: ModelLibraryClient = { getStatus: vi.fn().mockResolvedValue(snapshot), scan: vi.fn(), select: vi.fn(), cancel: vi.fn(), download: vi.fn().mockResolvedValue({ ...snapshot, downloads: [{ id: "download-1", model_id: "gpt-sovits-v2", state: "downloading", message: "准备下载", downloaded_bytes: 0, total_bytes: 0, path: "" }] }) };
  const { result } = renderHook(() => useModelLibrary(client, "token", vi.fn()), { wrapper: FeedbackProvider });
  await act(async () => {});
  await act(async () => { await result.current.run("download", "gpt-sovits-v2"); });
  expect(screen.getByRole("dialog")).toHaveTextContent("下载请求已提交");
  expect(screen.getByRole("dialog")).not.toHaveTextContent("下载完成");
});

it("shows a resource business error without claiming the desktop operation succeeded", async () => {
  const client = { getSnapshot: vi.fn().mockResolvedValue(resourceSnapshot()), desktop: vi.fn().mockResolvedValue({ type: "error", code: "vts_unavailable", message: "VTube Studio 未连接" }) } as unknown as ResourcesClient;
  const { result } = renderHook(() => useResourcesController(client, {} as ServerClient), { wrapper: FeedbackProvider });
  await act(async () => {});
  await act(async () => { await result.current.refreshModels(); });
  expect(screen.getByRole("dialog")).toHaveClass("is-error");
  expect(screen.getByRole("dialog")).toHaveTextContent("VTube Studio 未连接");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

it("shows successful OBS recording and an error after a refused stop", async () => {
  const client = { baseUrl: "test", getStatus: vi.fn().mockResolvedValue({ connected: true, recording: false, current_scene: "直播", scenes: ["直播"] }), execute: vi.fn().mockResolvedValueOnce({ connected: true, recording: true, current_scene: "直播", scenes: ["直播"] }).mockRejectedValueOnce(new Error("OBS 断线")) };
  render(<FeedbackProvider><ObsPanel client={client} /></FeedbackProvider>);
  await act(async () => {});
  fireEvent.click(screen.getByRole("button", { name: "开始录制" }));
  expect(await screen.findByRole("dialog")).toHaveTextContent("录制已开始");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  fireEvent.click(screen.getByRole("button", { name: "停止录制" }));
  expect(await screen.findByRole("dialog")).toHaveTextContent("OBS 断线");
});

it("shows LLM validation and saved configuration restart requirements in dialogs", async () => {
  const snapshot = { settings: { provider: "custom", api_format: "openai_chat" as const, base_url: "http://localhost/v1", model: "local", mode: "local" as const, timeout_seconds: 30, max_tokens: 512, json_mode: true }, key_configured: false, restart_required: false, active_model: "local", storage_available: true };
  const client: LlmClient = { baseUrl: "test", getSettings: vi.fn().mockResolvedValue(snapshot), saveSettings: vi.fn().mockResolvedValue({ ...snapshot, restart_required: true }), testSettings: vi.fn() };
  render(<FeedbackProvider><LlmPanel client={client} /></FeedbackProvider>);
  await act(async () => {});
  fireEvent.change(screen.getByLabelText("模型名称"), { target: { value: "" } });
  fireEvent.click(screen.getByRole("button", { name: "保存配置" }));
  expect(screen.getByRole("dialog")).toHaveTextContent("模型名称");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  fireEvent.change(screen.getByLabelText("模型名称"), { target: { value: "local" } });
  fireEvent.click(screen.getByRole("button", { name: "保存配置" }));
  expect(await screen.findByRole("dialog")).toHaveTextContent("重启主服务");
});

it("shows rejected reference audio and invalid voice submissions in dialogs", async () => {
  const controller = { snapshot: resourceSnapshot(), pendingAction: null, voiceDeleteRetries: [], voicePreview: null } as unknown as VoiceController;
  render(<FeedbackProvider><VoicePanel controller={controller} /></FeedbackProvider>);
  fireEvent.change(screen.getByLabelText("参考音频"), { target: { files: [new File(["invalid"], "voice.wav", { type: "audio/wav" })] } });
  expect(await screen.findByRole("dialog")).toHaveClass("is-error");
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  fireEvent.submit(screen.getByRole("button", { name: "上传音色" }).closest("form")!);
  expect(screen.getByRole("dialog")).toHaveTextContent("音色");
});

it("shows missing character fields when a save is submitted", () => {
  const controller = { snapshot: resourceSnapshot(), pendingAction: null, models: [], hotkeys: [], hotkeyModelId: null, installedModels: null } as unknown as CharacterController;
  render(<FeedbackProvider><CharacterPanel controller={controller} /></FeedbackProvider>);
  fireEvent.submit(screen.getByRole("button", { name: "保存角色" }).closest("form")!);
  expect(screen.getByRole("dialog")).toHaveTextContent("角色");
});
