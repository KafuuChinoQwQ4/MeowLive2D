import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it } from "vitest";
import { ObsPanel } from "./ObsPanel";
import { createObsClient } from "../../services/server/obs";

it("only reads status on mount and refreshes scene and recording state after explicit actions", async () => {
  const operations: string[] = [];
  let scene = "主场景", recording = false;
  const client = createObsClient({ fetcher: async (_url, init) => {
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
  const client = createObsClient({ fetcher: async () => {
    if (++count === 1) return Response.json({ connected: true, recording: false, current_scene: "主场景", scenes: ["主场景"] });
    throw new Error("offline");
  } });
  render(<ObsPanel client={client} />);
  await userEvent.click(await screen.findByRole("button", { name: "开始录制" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("请刷新状态确认");
  expect(screen.getByRole("button", { name: "开始录制" })).toBeDisabled();
});
