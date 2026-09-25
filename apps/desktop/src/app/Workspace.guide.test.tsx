import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it } from "vitest";
import { Workspace, type WorkspaceProps } from "./Workspace";

afterEach(() => window.history.replaceState(null, "", "/"));

const pages: WorkspaceProps["pages"] = {
  speech: <label>播报草稿<input /></label>, resources: <p>角色工作区</p>,
  agent: <p>互动工作区</p>, viewers: <p>观众工作区</p>, llm: <p>模型连接工作区</p>,
  "agent-observability": <p>Agent 观察工作区</p>,
  live: <p>直播工作区</p>, obs: <p>录制工作区</p>, training: <p>训练工作区</p>,
};

function workspace(ready = true) {
  return <Workspace pages={pages} overview={<p>运行开关</p>} setup={<p>环境检查</p>} ready={ready} status={[]} />;
}

it("opens a guide deep link before the main service is ready and can navigate to setup", async () => {
  window.history.replaceState(null, "", "#guide");
  render(workspace(false));
  expect(screen.getByRole("heading", { level: 1, name: "使用指南" })).toBeVisible();
  expect(screen.getByRole("heading", { name: "新手入门" })).toBeVisible();
  expect(screen.queryByRole("heading", { name: "先准备好主服务" })).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "检查环境与模型" }));
  expect(screen.getByText("环境检查")).toBeVisible();
  expect(screen.getByRole("heading", { name: "数据库与可选功能" })).toBeVisible();
  expect(screen.getByRole("link", { name: "下载 Docker Desktop（Windows）" })).toHaveAttribute("href", "https://docs.docker.com/desktop/setup/install/windows-install/");
  expect(window.location.hash).toBe("#setup");
});

it("returns from the guide to the current feature without losing its draft", async () => {
  window.history.replaceState(null, "", "#speech");
  render(workspace());
  await userEvent.type(screen.getByRole("textbox", { name: "播报草稿" }), "保留我的播报");
  await userEvent.click(screen.getByRole("link", { name: "使用指南" }));
  await userEvent.click(screen.getByText("语音播报", { selector: "summary span" }));
  const topic = screen.getByRole("region", { name: "语音播报使用方法" });
  await userEvent.click(within(topic).getByRole("button", { name: "前往语音播报" }));
  expect(screen.getByRole("textbox", { name: "播报草稿" })).toHaveValue("保留我的播报");
  expect(screen.getByRole("link", { name: "语音播报" })).toHaveAttribute("aria-current", "page");
  await act(async () => {
    window.history.replaceState(null, "", "#guide");
    window.dispatchEvent(new PopStateEvent("popstate"));
  });
  expect(screen.getByRole("heading", { level: 1, name: "使用指南" })).toBeVisible();
});
