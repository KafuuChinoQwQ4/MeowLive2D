import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ResourcesPanel } from "./ResourcesPanel";
import type { ResourcesClient } from "../../services/server/resources";
import type { ServerClient } from "../../services/server";
import { resourceSnapshot } from "../../test/resource-fixtures";

describe("资源管理恢复", () => {
  it("当前角色在 VTS 重启后仍可重新加载", async () => {
    const snapshot = resourceSnapshot();
    snapshot.active_character_id = snapshot.characters[0]!.id;
    const client = { getSnapshot: vi.fn().mockResolvedValue(snapshot), selectCharacter: vi.fn().mockResolvedValue(snapshot) } as unknown as ResourcesClient;
    render(<ResourcesPanel resourceClient={client} speechClient={{} as ServerClient} />);
    const button = await screen.findByRole("button", { name: "重新加载" });
    expect(button).toBeEnabled();
    fireEvent.click(button);
    await waitFor(() => expect(client.selectCharacter).toHaveBeenCalled());
  });

  it("VTS 刷新失败仍显示已经安装成功的模型", async () => {
    const client = {
      getSnapshot: vi.fn().mockResolvedValue(resourceSnapshot()),
      desktop: vi.fn().mockResolvedValueOnce({ type: "model_imported", model_name: "魔女", model_file: "魔女.model3.json", files: 22, bytes: 44670629, restart_required: true }).mockRejectedValueOnce(new Error("VTS 尚未启动")),
    } as unknown as ResourcesClient;
    render(<ResourcesPanel resourceClient={client} speechClient={{} as ServerClient} />);
    fireEvent.click(await screen.findByRole("button", { name: "从本机导入模型" }));
    expect(await screen.findByText(/已导入 魔女/)).toBeVisible();
    expect(await screen.findByText(/VTS 尚未启动/)).toBeVisible();
  });
});
