import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { App } from "./App";
import { agentStatus } from "../test/agent-fixtures";
import { resourceSnapshot, voice } from "../test/resource-fixtures";
import { deferred, jsonResponse, serverStatus } from "../test/server-fixtures";

afterEach(() => { vi.unstubAllGlobals(); window.history.replaceState(null, "", "/"); });

function server(resourceSource = () => resourceSnapshot()) {
  const saved = agentStatus();
  const fetcher = vi.fn<typeof fetch>(async (url, init) => {
    const path = new URL(String(url)).pathname;
    if (path === "/api/admin/session") return jsonResponse({ enabled: false, authenticated: false });
    if (path === "/api/resources") return jsonResponse(resourceSource());
    if (path === "/api/agent/settings" && init?.body) return jsonResponse(agentStatus({ settings: JSON.parse(String(init.body)) }));
    if (path === "/api/agent") return jsonResponse(saved);
    return jsonResponse(serverStatus());
  });
  vi.stubGlobal("fetch", fetcher);
  return fetcher;
}

it("groups Agent pages and speech pages, and puts related editors on the same pages", async () => {
  server();
  render(<App />);
  const nav = await screen.findByRole("navigation", { name: "功能导航" });
  expect(within(nav).getByText("Agent 与智能")).toBeVisible();
  expect(within(nav).getByText("声音与播报")).toBeVisible();
  await userEvent.click(within(nav).getByRole("link", { name: "角色与人物卡" }));
  expect(await screen.findByRole("heading", { name: "角色管理" })).toBeVisible();
  expect(await screen.findByRole("heading", { name: "主播人物卡" })).toBeVisible();
  await userEvent.click(within(nav).getByRole("link", { name: "声音训练" }));
  expect(await screen.findByRole("heading", { name: "音色管理" })).toBeVisible();
  expect(screen.getByRole("heading", { name: "训练状态" })).toBeVisible();
});

it("refreshes character voice choices after changing sounds on the training page", async () => {
  let resources = resourceSnapshot();
  server(() => resources);
  render(<App />);
  await userEvent.click(await screen.findByRole("link", { name: "角色与人物卡" }));
  await screen.findByRole("heading", { name: "角色管理" });
  await userEvent.click(screen.getByRole("link", { name: "声音训练" }));
  resources = resourceSnapshot({ voices: [voice(), voice({ id: "voice-2", name: "新声音" })] });
  await userEvent.click(screen.getByRole("link", { name: "角色与人物卡" }));
  expect(await screen.findByRole("option", { name: "新声音" })).toBeInTheDocument();
});

it("ignores a late resource response after returning from the sound page", async () => {
  const oldResponse = deferred<Response>();
  let resourceRequests = 0;
  vi.stubGlobal("fetch", vi.fn<typeof fetch>(async url => {
    const path = new URL(String(url)).pathname;
    if (path === "/api/admin/session") return jsonResponse({ enabled: false, authenticated: false });
    if (path === "/api/resources") return ++resourceRequests === 1 ? oldResponse.promise
      : jsonResponse(resourceSnapshot({ voices: [voice(), voice({ id: "voice-2", name: "新声音" })] }));
    if (path === "/api/agent") return jsonResponse(agentStatus());
    return jsonResponse(serverStatus());
  }));
  render(<App />);
  await userEvent.click(await screen.findByRole("link", { name: "角色与人物卡" }));
  await userEvent.click(screen.getByRole("link", { name: "声音训练" }));
  await userEvent.click(screen.getByRole("link", { name: "角色与人物卡" }));
  expect(await screen.findByRole("option", { name: "新声音" })).toBeInTheDocument();
  await act(async () => { oldResponse.resolve(jsonResponse(resourceSnapshot())); });
  expect(screen.getByRole("option", { name: "新声音" })).toBeInTheDocument();
});

it("saving a persona from the character page preserves the latest interaction settings", async () => {
  const fetcher = server();
  render(<App />);
  await userEvent.click(await screen.findByRole("link", { name: "角色与人物卡" }));
  const editor = await screen.findByRole("region", { name: "主播人物卡" });
  await userEvent.clear(within(editor).getByRole("textbox", { name: "核心身份" }));
  await userEvent.type(within(editor).getByRole("textbox", { name: "核心身份" }), "新角色人设");
  await userEvent.click(within(editor).getByRole("button", { name: "保存人物卡并暂停 Agent" }));
  await waitFor(() => expect(fetcher.mock.calls.some(([url, init]) =>
    String(url).endsWith("/api/agent/settings")
      && JSON.parse(String(init?.body)).persona === "新角色人设"
      && JSON.parse(String(init?.body)).topic === agentStatus().settings.topic,
  )).toBe(true));
});
