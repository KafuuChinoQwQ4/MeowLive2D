import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { ServerClient } from "../../services/server";
import type { ResourcesClient } from "../../services/server/resources";
import { character, hotkey, model, resourceSnapshot } from "../../test/resource-fixtures";
import { ResourcesPanel } from "../../app/resources";

function setupClients() {
  const initial = resourceSnapshot({ active_character_id: null });
  const saved = resourceSnapshot({ characters: [character({ name: "更新角色" })] });
  const validated = resourceSnapshot({ characters: [character({ name: "更新角色", mappings: [
    { intent: "挥手", hotkey_id: "hotkey-wave", fallback_hotkey_id: "hotkey-idle", validated: true },
  ] })] });
  const resourceClient: ResourcesClient = {
    baseUrl: "http://localhost:19600",
    getSnapshot: vi.fn().mockResolvedValue(initial),
    createVoice: vi.fn().mockResolvedValue(initial),
    selectVoice: vi.fn().mockResolvedValue(initial),
    deleteVoice: vi.fn().mockResolvedValue(initial),
    deleteCharacter: vi.fn().mockResolvedValue(initial),
    saveCharacter: vi.fn().mockResolvedValue(saved),
    selectCharacter: vi.fn().mockResolvedValue(initial),
    previewCharacter: vi.fn().mockResolvedValue(validated),
    desktop: vi.fn().mockImplementation(async (operation) => {
      if (operation.type === "list_hotkeys") return { type: "hotkeys", model_id: operation.model_id, hotkeys: [hotkey(), hotkey({ id: "hotkey-idle", name: "Idle" })] };
      if (operation.type === "import_model") return { type: "model_imported", model_name: "New Cat", model_file: "NewCat.model3.json", files: 4, bytes: 1024, restart_required: true };
      return { type: "models", models: [model(), model({ id: "model-new", name: "New Cat" })] };
    }),
  };
  const speechClient: ServerClient = { baseUrl: "", getStatus: vi.fn(), submitSpeech: vi.fn(), stop: vi.fn() };
  return { resourceClient, speechClient };
}

describe("角色管理", () => {
  it("已安装模型删除需要确认，成功刷新本地列表", async () => {
    const user = userEvent.setup();
    const pair = setupClients();
    const id = "a".repeat(64);
    let deleted = false;
    pair.resourceClient.desktop = vi.fn<ResourcesClient["desktop"]>(async operation => {
      if (operation.type === "delete_imported_model") {
        deleted = true;
        return { type: "model_deleted", id, restart_required: true };
      }
      return { type: "imported_models", models: deleted ? [] : [{ id, name: "导入猫咪", model_id: "model-new" }] };
    });
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
    render(<ResourcesPanel {...pair} />);
    await user.click(await screen.findByRole("button", { name: "管理已安装模型" }));
    await user.click(await screen.findByRole("button", { name: "删除模型 导入猫咪" }));
    expect(deleted).toBe(false);
    confirm.mockReturnValue(true);
    await user.click(screen.getByRole("button", { name: "删除模型 导入猫咪" }));
    await waitFor(() => expect(screen.queryByRole("button", { name: "删除模型 导入猫咪" })).not.toBeInTheDocument());
    expect(pair.resourceClient.desktop).toHaveBeenCalledWith({ type: "delete_imported_model", id }, expect.any(AbortSignal));
    expect(screen.getByText(/已删除模型.*重启 VTube Studio/)).toBeVisible();
    confirm.mockRestore();
  });
  it("删除正在编辑的角色后清空表单并保留声音资源", async () => {
    const user = userEvent.setup();
    const pair = setupClients();
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
    pair.resourceClient.deleteCharacter = vi.fn().mockResolvedValue(resourceSnapshot({ characters: [], active_character_id: null }));
    render(<ResourcesPanel {...pair} />);
    await user.click(await screen.findByRole("button", { name: "编辑" }));
    await user.click(screen.getByRole("button", { name: "删除角色 小猫主播" }));
    expect(pair.resourceClient.deleteCharacter).toHaveBeenCalledWith({ id: "character-1" }, expect.any(AbortSignal));
    await waitFor(() => expect(screen.getByLabelText("角色名称")).toHaveValue(""));
    expect(screen.getByText("温柔旁白", { selector: "strong" })).toBeVisible();
    confirm.mockRestore();
  });
  it("新角色先等待用户选择音色，不预选配置默认音色", async () => {
    const user = userEvent.setup();
    const pair = setupClients();
    render(<ResourcesPanel {...pair} />);
    await screen.findByRole("heading", { name: "角色管理" });
    await user.click(screen.getByRole("button", { name: "刷新 VTS 模型" }));
    await user.type(screen.getByLabelText("角色名称"), "新角色");
    await user.selectOptions(screen.getByLabelText("VTS 模型"), "model-1");

    expect(screen.getByLabelText("角色音色")).toHaveValue("");
    expect(screen.getByRole("button", { name: "保存角色" })).toBeDisabled();
    await user.selectOptions(screen.getByLabelText("角色音色"), "voice-1");
    expect(screen.getByRole("button", { name: "保存角色" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "新建角色" }));
    expect(screen.getByLabelText("角色音色")).toHaveValue("");
  });

  it("导入使用本机选择器操作，并在完成后刷新模型列表", async () => {
    const user = userEvent.setup();
    const pair = setupClients();
    render(<ResourcesPanel {...pair} />);
    await screen.findByRole("heading", { name: "角色管理" });

    await user.click(screen.getByRole("button", { name: "从本机导入模型" }));

    expect(await screen.findByText(/已导入 New Cat.*请重启 VTube Studio/)).toBeVisible();
    expect(pair.resourceClient.desktop).toHaveBeenNthCalledWith(1, { type: "import_model" }, expect.any(AbortSignal));
    expect(pair.resourceClient.desktop).toHaveBeenNthCalledWith(2, { type: "list_models" }, expect.any(AbortSignal));
    expect(screen.queryByRole("textbox", { name: /路径/ })).not.toBeInTheDocument();
  });

  it("编辑映射后先保存，再通过角色预览接口验证热键", async () => {
    const user = userEvent.setup();
    const pair = setupClients();
    render(<ResourcesPanel {...pair} />);
    await screen.findByRole("heading", { name: "角色管理" });
    await user.click(screen.getByRole("button", { name: "编辑" }));
    await user.click(screen.getByRole("button", { name: "刷新热键" }));
    await waitFor(() => expect(pair.resourceClient.desktop).toHaveBeenCalledWith(
      { type: "list_hotkeys", model_id: "model-1" }, expect.any(AbortSignal),
    ));

    const name = screen.getByLabelText("角色名称");
    await user.clear(name);
    await user.type(name, "更新角色");
    await user.selectOptions(screen.getByLabelText("备用热键"), "hotkey-idle");
    expect(screen.getByRole("button", { name: "预览验证" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "保存角色" }));

    await waitFor(() => expect(pair.resourceClient.saveCharacter).toHaveBeenCalledWith(expect.objectContaining({
      id: "character-1",
      name: "更新角色",
      mappings: [{ intent: "挥手", hotkey_id: "hotkey-wave", fallback_hotkey_id: "hotkey-idle", validated: false }],
    }), expect.any(AbortSignal)));
    await user.click(screen.getByRole("button", { name: "预览验证" }));
    expect(pair.resourceClient.previewCharacter).toHaveBeenCalledWith(
      { character_id: "character-1", intent: "挥手" }, expect.any(AbortSignal),
    );
    expect(await screen.findByText("已验证")).toBeVisible();
  });

  it("选择角色通过资源接口加载，并仅在成功快照中标为当前角色", async () => {
    const user = userEvent.setup();
    const pair = setupClients();
    pair.resourceClient.selectCharacter = vi.fn().mockResolvedValue(resourceSnapshot({ active_character_id: "character-1" }));
    render(<ResourcesPanel {...pair} />);
    await screen.findByRole("heading", { name: "角色管理" });

    expect(screen.queryByText("当前角色")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "选择并加载" }));
    expect(pair.resourceClient.selectCharacter).toHaveBeenCalledWith({ id: "character-1" }, expect.any(AbortSignal));
    expect(await screen.findByText("当前角色")).toBeVisible();
  });
});
