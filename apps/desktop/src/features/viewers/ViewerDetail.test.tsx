import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { ServerRequestError } from "../../services/server/responses";
import { ViewerDetail } from "./ViewerDetail";
const detail = {
  viewer_id: "real-v",
  affinity_milli: 200,
  familiarity_milli: 1000,
  observed_days: 1,
  observed_sessions: 2,
  last_seen_at_ms: 1,
  medal_level: null,
  guard_level: null,
  gifts: [],
  ledger: [],
};
function clients() {
  return {
    companionship: {
      detail: vi.fn().mockResolvedValue(detail),
      status: vi
        .fn()
        .mockResolvedValue({
          durable_receipts: true,
          failed_receipts: [],
          pending_receipts: 0,
          failed_receipt_attempts: 0,
        }),
      adjust: vi
        .fn()
        .mockRejectedValueOnce(new Error("network"))
        .mockResolvedValue({ record_id: "r" }),
      reverse: vi.fn(),
      confirmGift: vi.fn(),
    },
    memory: {
      list: vi.fn().mockResolvedValue({ memories: [] }),
      status: vi.fn().mockResolvedValue({
        pending: 1,
        running: 0,
        failed: 2,
        embedding_pending: 0,
        embedding_failed: 0,
      }),
      mutate: vi.fn(),
      retry: vi.fn(),
      rebuildVectors: vi.fn(),
    },
  };
}
it("retries identical adjustment with same key but new inputs get a new key", async () => {
  const c = clients();
  render(<ViewerDetail viewerId="real-v" {...c} />);
  await screen.findByText("好感度 0.2 / 100");
  await userEvent.type(screen.getByLabelText("调整毫分"), "200");
  await userEvent.type(screen.getByLabelText("调整原因"), "核实");
  await userEvent.click(screen.getByRole("button", { name: "提交调整" }));
  await screen.findByRole("alert");
  await userEvent.click(screen.getByRole("button", { name: "提交调整" }));
  expect(c.companionship.adjust.mock.calls[0][1].request_key).toBe(
    c.companionship.adjust.mock.calls[1][1].request_key,
  );
  await userEvent.clear(screen.getByLabelText("调整毫分"));
  await userEvent.type(screen.getByLabelText("调整毫分"), "300");
  await userEvent.click(screen.getByRole("button", { name: "提交调整" }));
  expect(c.companionship.adjust.mock.calls[2][1].request_key).not.toBe(
    c.companionship.adjust.mock.calls[1][1].request_key,
  );
});
it("deletes only after an explicit reason and removes the memory after success", async () => {
  const c = clients();
  const m = {
    id: "m",
    viewer_id: "real-v",
    key: "preference",
    kind: "preference",
    value: "猫",
    status: "long_term",
    version: 2,
    locked: false,
    deleted: false,
    expires_at_ms: null,
    evidence: [
      {
        source: "bilibili",
        event_id: "e",
        quote: "我喜欢猫",
        occurred_at_ms: 1,
      },
    ],
  };
  c.memory.list
    .mockResolvedValueOnce({ memories: [m] })
    .mockResolvedValue({ memories: [] });
  c.memory.mutate.mockResolvedValue({ record_id: "m" });
  render(<ViewerDetail viewerId="real-v" {...c} />);
  await screen.findByRole("article", { name: "记忆 猫" });
  expect(screen.getByRole("button", { name: "确认删除记忆" })).toBeDisabled();
  await userEvent.type(screen.getByLabelText("记忆操作原因"), "原文错误");
  await userEvent.click(screen.getByRole("button", { name: "确认删除记忆" }));
  expect(c.memory.mutate).toHaveBeenCalledWith(
    "real-v",
    "m",
    expect.objectContaining({
      expected_version: 2,
      operation: "delete",
      reason: "原文错误",
    }),
    expect.any(AbortSignal),
  );
  expect(
    screen.queryByRole("article", { name: "记忆 猫" }),
  ).not.toBeInTheDocument();
});
it("ignores old detail responses after viewer selection changes", async () => {
  const c = clients();
  let release!: (v: typeof detail) => void;
  c.companionship.detail
    .mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          release = resolve;
        }),
    )
    .mockResolvedValue({ ...detail, viewer_id: "next", affinity_milli: 500 });
  const rendered = render(<ViewerDetail viewerId="old" {...c} />);
  rendered.rerender(<ViewerDetail viewerId="next" {...c} />);
  await screen.findByText("好感度 0.5 / 100");
  release(detail);
  await Promise.resolve();
  expect(screen.queryByText("好感度 0.2 / 100")).not.toBeInTheDocument();
});
it("requires refresh after a version conflict before another write", async () => {
  const c = clients();
  c.companionship.adjust
    .mockReset()
    .mockRejectedValue(new ServerRequestError("conflict", "conflict", 409));
  render(<ViewerDetail viewerId="real-v" {...c} />);
  await screen.findByText("好感度 0.2 / 100");
  await userEvent.type(screen.getByLabelText("调整毫分"), "200");
  await userEvent.type(screen.getByLabelText("调整原因"), "核实");
  await userEvent.click(screen.getByRole("button", { name: "提交调整" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("版本冲突");
  expect(screen.getByRole("button", { name: "提交调整" })).toBeDisabled();
  await userEvent.click(screen.getByRole("button", { name: "刷新详情与任务" }));
  await screen.findByText("好感度 0.2 / 100");
  expect(screen.getByRole("button", { name: "提交调整" })).not.toBeDisabled();
});
it("refreshing after an uncertain write does not discard the retry key", async () => {
  const c = clients();
  render(<ViewerDetail viewerId="real-v" {...c} />);
  await screen.findByText("好感度 0.2 / 100");
  await userEvent.type(screen.getByLabelText("调整毫分"), "200");
  await userEvent.type(screen.getByLabelText("调整原因"), "核实");
  await userEvent.click(screen.getByRole("button", { name: "提交调整" }));
  await screen.findByRole("alert");
  await userEvent.click(screen.getByRole("button", { name: "刷新详情与任务" }));
  await screen.findByText("好感度 0.2 / 100");
  await userEvent.click(screen.getByRole("button", { name: "提交调整" }));
  expect(c.companionship.adjust.mock.calls[0][1].request_key).toBe(
    c.companionship.adjust.mock.calls[1][1].request_key,
  );
});
it("memory recovery requires a reason and sends a stable request key", async () => {
  const c = clients();
  c.memory.retry.mockResolvedValue({ updated: true });
  render(<ViewerDetail viewerId="real-v" {...c} />);
  await screen.findByText("好感度 0.2 / 100");
  expect(
    screen.getByRole("button", { name: "重试失败记忆任务" }),
  ).toBeDisabled();
  await userEvent.type(screen.getByLabelText("记忆恢复操作原因"), "模型已恢复");
  await userEvent.click(
    screen.getByRole("button", { name: "重试失败记忆任务" }),
  );
  expect(c.memory.retry).toHaveBeenCalledWith(
    expect.objectContaining({
      reason: "模型已恢复",
      request_key: expect.any(String),
    }),
    expect.any(AbortSignal),
  );
});
it("does not offer reversal for a premerge ledger marked nonreversible", async () => {
  const c = clients();
  c.companionship.detail.mockResolvedValue({
    ...detail,
    ledger: [
      {
        ledger_id: "old",
        kind: "manual",
        computed_delta_milli: 200,
        applied_delta_milli: 200,
        reason: "合并前调整",
        actor: "admin",
        created_at_ms: 1,
        reversed_ledger_id: null,
        reversible: false,
      },
    ],
  });
  render(<ViewerDetail viewerId="real-v" {...c} />);
  await screen.findByText(/合并前调整/);
  expect(
    screen.queryByRole("button", { name: "撤销此项积分" }),
  ).not.toBeInTheDocument();
  expect(screen.getByText(/此记录不可直接撤销/)).toBeInTheDocument();
});
it("shows durable receipt recovery and individual failed speech IDs", async () => {
  const c = clients();
  c.companionship.status.mockResolvedValue({
    durable_receipts: true,
    failed_receipts: [{ speech_id: "failed-speech", attempts: 7 }],
    pending_receipts: 1,
    failed_receipt_attempts: 7,
  });
  render(<ViewerDetail viewerId="real-v" {...c} />);
  await screen.findByText("回执恢复：本地持久日志已启用");
  expect(screen.getByText(/failed-speech.*7/)).toBeInTheDocument();
});
