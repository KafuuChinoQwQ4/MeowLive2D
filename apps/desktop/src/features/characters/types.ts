import type { CharacterPreviewRequest, CharacterSaveRequest, DesktopResourceResult, ResourceSnapshot, VtsHotkey, VtsModel } from "@meowlive/contracts";

/** 页面通过显式能力传入角色状态；角色功能不依赖音色功能或应用 hooks。 */
export interface CharacterController {
  snapshot: ResourceSnapshot | null;
  pendingAction: string | null;
  models: VtsModel[];
  hotkeys: VtsHotkey[];
  hotkeyModelId: string | null;
  importResult: Extract<DesktopResourceResult, { type: "model_imported" }> | null;
  saveCharacter(request: CharacterSaveRequest): Promise<ResourceSnapshot | null>;
  selectCharacter(id: string): Promise<ResourceSnapshot | null>;
  previewCharacter(request: CharacterPreviewRequest): Promise<ResourceSnapshot | null>;
  refreshModels(): Promise<DesktopResourceResult | null>;
  refreshHotkeys(id: string): Promise<DesktopResourceResult | null>;
  importModel(): Promise<DesktopResourceResult | null>;
}
