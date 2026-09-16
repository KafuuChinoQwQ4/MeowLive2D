import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createLiveClient } from "../../services/server/live";
import { liveSnapshot } from "../../test/live-fixtures";
import { jsonResponse } from "../../test/server-fixtures";
import { ConnectionPanel } from "./ConnectionPanel";

describe("直播连接面板", () => {
  it("展示平台、房间、连接状态和全部运行计数", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(liveSnapshot({
      phase: "connected",
      room_id: "24680",
      accepted_events: 23,
      duplicate_events: 4,
      rejected_events: 2,
      reconnect_attempts: 1,
    })));

    render(<ConnectionPanel client={createLiveClient({ fetcher })} />);

    expect(await screen.findByRole("heading", { name: "直播间已连接" })).toBeVisible();
    const metrics = within(screen.getByLabelText("直播事件统计"));
    expect(metrics.getByText("哔哩哔哩")).toBeVisible();
    expect(metrics.getByText("24680")).toBeVisible();
    expect(metrics.getByText("23")).toBeVisible();
    expect(metrics.getByText("4")).toBeVisible();
    expect(metrics.getByText("2")).toBeVisible();
    expect(metrics.getByText("1")).toBeVisible();
  });

  it("未配置时解释原因且不能连接", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(liveSnapshot({ configured: false, phase: "disabled" })));
    render(<ConnectionPanel client={createLiveClient({ fetcher })} />);

    expect(await screen.findByRole("heading", { name: "直播接入未启用" })).toBeVisible();
    expect(screen.getByRole("button", { name: "连接直播间" })).toBeDisabled();
    expect(screen.getByText(/主服务配置/)).toBeVisible();
  });

  it("服务端正在断开时保持控制按钮禁用", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(liveSnapshot({ phase: "disconnecting" })));
    render(<ConnectionPanel client={createLiveClient({ fetcher })} />);

    expect(await screen.findByRole("button", { name: "正在断开…" })).toBeDisabled();
  });

  it.each(["disconnected", "failed"] as const)("%s 状态允许重新连接并采用操作响应", async (phase) => {
    const fetcher = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(jsonResponse(liveSnapshot({ phase })))
      .mockResolvedValueOnce(jsonResponse(liveSnapshot({ phase: "connecting" })));
    render(<ConnectionPanel client={createLiveClient({ fetcher })} />);

    fireEvent.click(await screen.findByRole("button", { name: "连接直播间" }));

    expect(await screen.findByRole("heading", { name: "正在连接直播间…" })).toBeVisible();
  });

  it.each(["connecting", "connected", "reconnecting"] as const)("%s 状态允许断开", async (phase) => {
    const fetcher = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(jsonResponse(liveSnapshot({ phase })))
      .mockResolvedValueOnce(jsonResponse(liveSnapshot({ phase: "disconnecting" })));
    render(<ConnectionPanel client={createLiveClient({ fetcher })} />);

    fireEvent.click(await screen.findByRole("button", { name: "断开直播间" }));

    expect(await screen.findByRole("heading", { name: "正在断开直播间…" })).toBeVisible();
    expect(screen.getByRole("button", { name: "正在断开…" })).toBeDisabled();
  });

  it("操作失败时展示原因且不伪造连接状态", async () => {
    const fetcher = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(jsonResponse(liveSnapshot()))
      .mockResolvedValueOnce(jsonResponse({ code: "platform_unavailable", message: "直播平台暂时不可用" }, 503));
    render(<ConnectionPanel client={createLiveClient({ fetcher })} />);

    fireEvent.click(await screen.findByRole("button", { name: "连接直播间" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("直播平台暂时不可用");
    expect(screen.getByRole("heading", { name: "直播间未连接" })).toBeVisible();
  });

  it("快照中的平台错误对用户可见", async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(jsonResponse(liveSnapshot({ phase: "failed", last_error: "平台鉴权失败" })));
    render(<ConnectionPanel client={createLiveClient({ fetcher })} />);

    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("平台鉴权失败"));
  });
});
