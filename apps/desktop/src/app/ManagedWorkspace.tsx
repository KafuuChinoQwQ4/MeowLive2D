import type { LauncherServiceId } from "@meowlive/contracts";
import { launcherStateLabel } from "../features/launcher/LauncherControls";
import { ModelLibraryPanel } from "../features/model-library/ModelLibraryPanel";
import { createModelLibraryClient } from "../services/model-library";
import { useState } from "react";
import { LauncherControls, useLauncher } from "../features/launcher";
import { createLauncherClient } from "../services/launcher";
import { Workspace, type WorkspaceProps } from "./Workspace";

export function ManagedWorkspace({ pages }: Pick<WorkspaceProps, "pages">) {
  const [client] = useState(createLauncherClient);
  const [modelClient] = useState(createModelLibraryClient);
  const controller = useLauncher(client);
  const state = (id: LauncherServiceId) => controller.snapshot?.services.find(service => service.id === id)?.state;
  const available = (id: LauncherServiceId) => !controller.stale && ["running", "external"].includes(state(id) ?? "");
  const label = (id: LauncherServiceId) => controller.stale ? "状态待确认" : launcherStateLabel(id, state(id) ?? "stopped");
  const titles = { server: "主服务", tts: "TTS", windows: "Windows 执行端" };
  return <Workspace pages={pages} setup={<ModelLibraryPanel client={modelClient} token={controller.stale ? null : controller.snapshot?.session_token ?? null} onSelected={controller.refreshQuietly} />} overview={<LauncherControls controller={controller} />} ready={available("server")}
    status={(["server", "tts", "windows"] as const).map(id => ({ label: `${titles[id]} · ${label(id)}`, available: controller.stale ? null : available(id) }))}
    notice={!available("tts") && <p className="availability-note">TTS 尚未就绪，可以先配置角色；播报前请在“启动与运行”打开 TTS 服务，再到“训练与离线”启用语音模型。</p>} />;
}
