import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { ViewerMergePanel } from "./ViewerMergePanel";
it("requires fresh preview plus explicit confirmation and selects canonical result", async () => {
  const summary = {
    viewer_id: "source",
    alias: "猫",
    identities: 1,
    events: 3,
    memories: 2,
    relationships: 0,
    affinity_milli: 200,
    familiarity_milli: 1000,
  };
  const client = {
    preview: vi
      .fn()
      .mockResolvedValue({
        source: summary,
        target: { ...summary, viewer_id: "target", alias: "另一个猫" },
        revision: 4,
        fingerprint: "fp",
        resulting_affinity_milli: 300,
        resulting_familiarity_milli: 1000,
        risks: ["来源将隐藏"],
      }),
    apply: vi
      .fn()
      .mockResolvedValue({ canonical_viewer_id: "target", revision: 5 }),
  };
  const merged = vi.fn();
  render(
    <ViewerMergePanel viewerId="source" client={client} onMerged={merged} />,
  );
  await userEvent.type(screen.getByLabelText("目标观众 UUID"), "target");
  await userEvent.click(screen.getByRole("button", { name: "预览身份合并" }));
  await screen.findByText("来源将隐藏");
  expect(screen.getByRole("button", { name: "执行身份合并" })).toBeDisabled();
  await userEvent.type(screen.getByLabelText("身份合并原因"), "已核实");
  await userEvent.click(screen.getByLabelText("已核对双方身份与合并风险"));
  await userEvent.click(screen.getByRole("button", { name: "执行身份合并" }));
  expect(merged).toHaveBeenCalledWith("target");
  expect(client.apply).toHaveBeenCalledWith(
    expect.objectContaining({
      fingerprint: "fp",
      expected_revision: 4,
      confirmed: true,
    }),
    expect.any(AbortSignal),
  );
});
it("changing the target invalidates a previously loaded preview", async () => {
  const summary = {
    viewer_id: "source",
    alias: "猫",
    identities: 1,
    events: 1,
    memories: 0,
    relationships: 0,
    affinity_milli: 0,
    familiarity_milli: 0,
  };
  const client = {
    preview: vi
      .fn()
      .mockResolvedValue({
        source: summary,
        target: { ...summary, viewer_id: "target" },
        revision: 1,
        fingerprint: "fp",
        resulting_affinity_milli: 0,
        resulting_familiarity_milli: 0,
        risks: ["风险"],
      }),
    apply: vi.fn(),
  };
  render(
    <ViewerMergePanel viewerId="source" client={client} onMerged={vi.fn()} />,
  );
  await userEvent.type(screen.getByLabelText("目标观众 UUID"), "target");
  await userEvent.click(screen.getByRole("button", { name: "预览身份合并" }));
  await screen.findByText("风险");
  await userEvent.type(screen.getByLabelText("目标观众 UUID"), "-changed");
  expect(
    screen.queryByRole("button", { name: "执行身份合并" }),
  ).not.toBeInTheDocument();
  expect(client.apply).not.toHaveBeenCalled();
});
