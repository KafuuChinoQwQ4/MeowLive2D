import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { agentEvent } from "../../test/agent-fixtures";
import { EventHistory } from "./EventHistory";

describe("Agent 事件历史", () => {
  it("展示事件内容、状态、关联语音与失败原因", () => {
    render(<EventHistory events={[
      agentEvent({ status: "playing", speech_id: "speech-7" }),
      agentEvent({
        event: { id: "gift-1", source: "simulator", viewer: "观众甲", kind: { type: "gift", name: "小鱼干", count: 3 } },
        status: "failed",
        error: "模型响应无效",
      }),
    ]} />);

    expect(screen.getByText("晚上好")).toBeVisible();
    expect(screen.getByText("播放中")).toBeVisible();
    expect(screen.getByText(/speech-7/)).toBeVisible();
    expect(screen.getByText(/小鱼干 × 3/)).toBeVisible();
    expect(screen.getByText("失败")).toBeVisible();
    expect(screen.getByText("模型响应无效")).toBeVisible();
  });
});
