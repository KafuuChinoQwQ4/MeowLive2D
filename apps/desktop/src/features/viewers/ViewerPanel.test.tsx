import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { ViewerPanel } from "./ViewerPanel";
import type { ViewerClient } from "../../services/server/viewers";
import { deferred } from "../../test/server-fixtures";
import { ServerRequestError } from "../../services/server/responses";

const viewers = Array.from({ length: 50 }, (_, index) => ({ viewer_id: `viewer-${index}`, current_alias: index === 0 ? "小猫" : `观众 ${index}`, alias_observed_at_ms: 1, identities: index === 0 ? [{ platform: "bilibili", namespace: "app-1", id_kind: "open_id", external_id: "viewer-1", last_confirmed_at_ms: 1 }] : [], aliases: [{ alias: index === 0 ? "旧昵称" : `曾用昵称 ${index}`, first_seen_at_ms: 1, last_seen_at_ms: 2 }] }));
const events = Array.from({ length: 50 }, (_, index) => ({ event_id: `event-${index}`, source: "bilibili", session_id: "session-1", viewer_id: `viewer-${index}`, viewer: index === 0 ? "小猫" : `观众 ${index}`, occurred_at_ms: 1, received_at_ms: 2, kind: { type: "gift" as const, name: "花", count: 3 }, gift_metadata: { price: 1000, paid: true, medal_level: 12, guard_level: 3 } }));
const client: ViewerClient = {
  baseUrl: "http://127.0.0.1:19995",
  listViewers: vi.fn().mockResolvedValue({ scope_id: "default", offset: 0, viewers }),
  listEvents: vi.fn().mockResolvedValue({ scope_id: "default", offset: 0, unconfirmed_events: 2, events }),
};

it("事件列表展示 SC 的金额、留言及进房行为", async () => {
  const eventClient: ViewerClient = {
    ...client,
    listEvents: vi.fn().mockResolvedValue({ scope_id: "default", offset: 0, unconfirmed_events: 0, events: [
      { ...events[0], gift_metadata: null, event_id: "sc", kind: { type: "super_chat", text: "继续加油", amount_cny: 50, start_at_ms: 1000, end_at_ms: 2000 } },
      { ...events[0], gift_metadata: null, event_id: "enter", kind: { type: "room_enter" } },
    ] }),
  };
  render(<ViewerPanel client={eventClient} />);
  expect(await screen.findByText("SC · 50 元 · 继续加油")).toBeVisible();
  expect(screen.getByText("进入直播间")).toBeVisible();
});

it("shows aliases, persisted events, and unconfirmed event count", async () => {
  render(<ViewerPanel client={client} />);

  expect((await screen.findAllByText("小猫"))).toHaveLength(2);
  expect(screen.getByText("旧昵称")).toBeInTheDocument();
  expect(screen.getByText("bilibili · open_id · viewer-1")).toBeInTheDocument();
  expect(screen.getAllByText("花 × 3")).toHaveLength(50);
  expect(screen.getByText("未确认接收缺口 2")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "下一页" }));
  expect(client.listViewers).toHaveBeenLastCalledWith(50, expect.any(AbortSignal));
});

it("分页请求进行时禁用翻页并清空旧页数据", async () => {
  const nextViewers = deferred<Awaited<ReturnType<ViewerClient["listViewers"]>>>();
  const nextEvents = deferred<Awaited<ReturnType<ViewerClient["listEvents"]>>>();
  const pagingClient: ViewerClient = {
    baseUrl: "http://127.0.0.1:19995",
    listViewers: vi.fn().mockResolvedValueOnce({ scope_id: "default", offset: 0, viewers }).mockReturnValueOnce(nextViewers.promise),
    listEvents: vi.fn().mockResolvedValueOnce({ scope_id: "default", offset: 0, unconfirmed_events: 2, events }).mockReturnValueOnce(nextEvents.promise),
  };
  render(<ViewerPanel client={pagingClient} />);

  await screen.findByText("旧昵称");
  await userEvent.click(screen.getByRole("button", { name: "下一页" }));

  expect(screen.getByRole("button", { name: "上一页" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "下一页" })).toBeDisabled();
  expect(screen.queryByText("旧昵称")).not.toBeInTheDocument();
  nextViewers.resolve({ scope_id: "default", offset: 50, viewers: [] });
  nextEvents.resolve({ scope_id: "default", offset: 50, unconfirmed_events: 2, events: [] });
  expect(await screen.findByText("尚无已识别观众。")).toBeInTheDocument();
});

it("为持久化存储故障显示配置提示", async () => {
  const unavailable: ViewerClient = {
    baseUrl: "http://127.0.0.1:19995",
    listViewers: vi.fn().mockRejectedValue(new ServerRequestError("viewer_store_unavailable", "unavailable")),
    listEvents: vi.fn().mockRejectedValue(new ServerRequestError("viewer_store_unavailable", "unavailable")),
  };
  render(<ViewerPanel client={unavailable} />);

  expect(await screen.findByRole("alert")).toHaveTextContent("持久化存储配置");
});

it("旧服务拒绝访问时提示更新服务而不是登录管理员", async () => {
  const unavailable: ViewerClient = {
    baseUrl: "http://127.0.0.1:19995",
    listViewers: vi.fn().mockRejectedValue(new ServerRequestError("auth_disabled", "disabled")),
    listEvents: vi.fn().mockRejectedValue(new ServerRequestError("auth_disabled", "disabled")),
  };
  render(<ViewerPanel client={unavailable} />);

  expect(await screen.findByRole("alert")).toHaveTextContent("更新并重启主服务");
});

it("读取失败后可在本页重新读取", async () => {
  const retrying: ViewerClient = {
    baseUrl: "http://127.0.0.1:19995",
    listViewers: vi.fn()
      .mockRejectedValueOnce(new ServerRequestError("viewer_store_unavailable", "unavailable"))
      .mockResolvedValueOnce({ scope_id: "default", offset: 0, viewers }),
    listEvents: vi.fn()
      .mockRejectedValueOnce(new ServerRequestError("viewer_store_unavailable", "unavailable"))
      .mockResolvedValueOnce({ scope_id: "default", offset: 0, unconfirmed_events: 2, events }),
  };
  render(<ViewerPanel client={retrying} />);

  expect(await screen.findByRole("alert")).toHaveTextContent("持久化存储配置");
  expect(screen.getByText("未确认接收缺口 未读取")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("button", { name: "重新读取" }));

  expect(await screen.findByText("旧昵称")).toBeInTheDocument();
});
it("opens management using the actual viewer ID",async()=>{
 render(<ViewerPanel client={client}/>);
 await userEvent.click(await screen.findByRole("button",{name:"管理 小猫"}));
 expect(screen.getByRole("region",{name:"观众管理详情"})).toHaveTextContent("viewer-0");
});
