import { useState, type ReactNode } from "react";
import { createLauncherClient, type LauncherClient } from "../../services/launcher";
import { useLauncher } from "./useLauncher";
import { LauncherControls } from "./LauncherControls";

export function LauncherPanel({ client, children }: { client?: LauncherClient; children?: ReactNode }) {
  const [defaultClient] = useState(createLauncherClient);
  const controller = useLauncher(client ?? defaultClient);
  const server = controller.snapshot?.services.find(service => service.id === "server");
  const tts = controller.snapshot?.services.find(service => service.id === "tts");
  const ready = !controller.stale && (server?.state === "running" || server?.state === "external");
  const ttsReady = !controller.stale && (tts?.state === "running" || tts?.state === "external");
  return <><LauncherControls controller={controller} />
    {ready ? <>{!ttsReady && <p className="availability-note">TTS 尚未就绪，播报前请等待语音引擎就绪。</p>}{children}</>
      : <p className="launcher-waiting">打开主服务开关后，从导航进入需要的功能。</p>}
  </>;
}
