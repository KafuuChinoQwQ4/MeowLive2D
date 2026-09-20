import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { RelationshipPanel } from "./RelationshipPanel";
it("shows SQL fallback and requires reason before confirmation", async () => {
  const fact = {
    id: "r",
    version: 2,
    source: { kind: "viewer", id: "v" },
    target: { kind: "viewer", id: "other" },
    kind: "acquaintance",
    confirmation: "claimed",
    evidence: [{ source: "bilibili", event_id: "e", quote: "我认识他" }],
    expires_at_ms: null,
    deleted: false,
  };
  const client = {
    list: vi.fn().mockResolvedValue({ relationships: [fact], degraded: true }),
    status: vi
      .fn()
      .mockResolvedValue({
        pending: 1,
        leased: 0,
        failed: 0,
        oldest_pending_age_ms: 2,
        connected: false,
      }),
    create: vi.fn(),
    change: vi.fn().mockResolvedValue(fact),
    rebuild: vi.fn(),
  };
  render(<RelationshipPanel viewerId="v" client={client} />);
  await screen.findByText("图服务降级：使用 SQL 已核对关系");
  expect(screen.getByRole("button", { name: "确认关系" })).toBeDisabled();
  await userEvent.type(screen.getByLabelText("关系操作原因"), "双方确认");
  await userEvent.click(screen.getByRole("button", { name: "确认关系" }));
  expect(client.change).toHaveBeenCalledWith(
    "r",
    expect.objectContaining({
      expected_version: 2,
      action: "confirm",
      reason: "双方确认",
    }),
    expect.any(AbortSignal),
  );
});
