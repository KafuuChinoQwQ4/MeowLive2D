import { WindowChrome } from "./WindowChrome";
import { hasNativeWindow } from "../services/desktop/window";
import type { LauncherServiceId } from "@meowlive/contracts";
import { DesktopLauncherControls } from "../features/launcher/DesktopLauncherControls";
import { listenForExitBlocked } from "../services/desktop/lifecycle";
import { AppUpdatePanel } from "../features/updates/AppUpdatePanel";
import { DesktopEnvironmentPanel } from "../features/desktop-environment/DesktopEnvironmentPanel";
import { SpeechPanel } from "../features/live";
import { AgentPanel } from "../features/agent";
import { AgentObservabilityPanel } from "../features/agent-observability";
import { LlmPanel } from "../features/llm";
import { ViewerPanel } from "../features/viewers";
import { ConnectionPanel } from "../features/connections";
import { TrainingPanel } from "../features/training";
import { ResourcesPanel } from "./resources";
import { useEffect, useMemo, useRef, useState } from "react";
import { getDesktopStatus, setDesktopServiceEnabled, type DesktopStatus } from "../services/desktop";
import { createServerClient } from "../services/server";
import { createAgentClient } from "../services/server/agent";
import { createLiveClient } from "../services/server/live";
import { createResourceClient } from "../services/server/resources";
import { createTrainingClient } from "../services/server/training";
import { createObsClient } from "../services/server/obs";
import { createLlmClient } from "../services/server/llm";
import { createLlmRuntimeClient } from "../services/server/llm-runtime";
import { createViewerClient } from "../services/server/viewers";
import { createAdminSessionClient } from "../services/server/auth";
import { createAgentObservabilityClient } from "../services/server/agent-observability";
import { ObsPanel } from "../features/obs";
import { ManagedWorkspace } from "./ManagedWorkspace";
import { Workspace, type WorkspaceProps } from "./Workspace";
import { FeedbackProvider, useFeedback } from "./feedback/OperationFeedback";
import { RuntimeLogsPanel } from "../features/logs/RuntimeLogsPanel";
import { createRuntimeLogClient } from "../services/server/logs";
import { createLauncherLogClient } from "../services/launcher/logs";

export function App() {
  return <FeedbackProvider><div className={`application-frame${hasNativeWindow() ? " is-native" : ""}`}><WindowChrome /><AppContent /></div></FeedbackProvider>;
}

function AppContent() {
  const feedback = useFeedback();
  const [desktop, setDesktop] = useState<DesktopStatus | null | undefined>(undefined);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const serviceAction = useRef(false);
  const [serviceBusy, setServiceBusy] = useState(false);
  const [environmentBusy, setEnvironmentBusy] = useState(false);
  const [soundRevision, setSoundRevision] = useState(0);
  useEffect(() => {
    let cancelled = false;
    let retryRequested = attempt > 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const update = async () => {
      try {
        const status = await getDesktopStatus();
        if (cancelled) return;
        if (serviceAction.current) { timer = setTimeout(() => void update(), 3_000); return; }
        setDesktop(status);
        setError(null);
        feedback.clearIssue("desktop:configuration");
        if (status?.runtime.last_error) feedback.reportIssue("desktop:runtime", "桌面执行端运行失败", status.runtime.last_error);
        else feedback.clearIssue("desktop:runtime");
        if (status?.server.last_error) feedback.reportIssue("desktop:server", "主服务未就绪", status.server.last_error);
        else feedback.clearIssue("desktop:server");
        if (retryRequested) { retryRequested = false; feedback.success("桌面连接已恢复", "已读取桌面配置。"); }
        if (status) timer = setTimeout(() => void update(), 3_000);
      } catch {
        if (!cancelled) {
          setError("桌面配置读取失败，请检查执行端状态后重试。");
          if (retryRequested) { retryRequested = false; feedback.error("重试桌面连接失败", "请检查执行端状态后重试。"); }
          else feedback.reportIssue("desktop:configuration", "桌面配置读取失败", "请检查执行端状态后重试。");
          timer = setTimeout(() => void update(), 3_000);
        }
      }
    };
    void update();
    return () => { cancelled = true; clearTimeout(timer); };
  }, [attempt, feedback]);
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listenForExitBlocked(message => feedback.error("暂时无法关闭 App", message)).then(stop => { if (disposed) stop(); else unlisten = stop; }).catch(() => {});
    return () => { disposed = true; unlisten?.(); };
  }, [feedback]);
  const baseUrl = desktop?.server_url;
  const clients = useMemo(() => ({
    speech: createServerClient({ baseUrl }), agent: createAgentClient({ baseUrl }),
    live: createLiveClient({ baseUrl }), resources: createResourceClient({ baseUrl }),
    training: createTrainingClient({ baseUrl }), obs: createObsClient({ baseUrl }),
    llm: createLlmClient({ baseUrl }), runtime: createLlmRuntimeClient({ baseUrl }), viewers: createViewerClient({ baseUrl }),
    auth: createAdminSessionClient({ baseUrl }), observability: createAgentObservabilityClient({ baseUrl }), logs: createRuntimeLogClient({ baseUrl }), launcherLogs: import.meta.env.VITE_MEOWLIVE_LAUNCHER === "true" ? createLauncherLogClient() : undefined,
  }), [baseUrl]);
  const panels: WorkspaceProps["pages"] = {
    live: <ConnectionPanel client={clients.live} />,
    obs: <ObsPanel client={clients.obs} />,
    resources: <ResourcesPanel resourceClient={clients.resources} speechClient={clients.speech} trainingClient={clients.training} agentClient={clients.agent} />,
    training: <div className="sound-workspace"><ResourcesPanel mode="voices" refreshToken={soundRevision} resourceClient={clients.resources} speechClient={clients.speech} trainingClient={clients.training} /><TrainingPanel client={clients.training} resources={clients.resources} onResourcesChanged={() => setSoundRevision(value => value + 1)} /></div>,
    speech: <SpeechPanel client={clients.speech} />,
    agent: <AgentPanel client={clients.agent} runtimeClient={clients.runtime} />,
    "agent-observability": <AgentObservabilityPanel client={clients.observability} />,
    viewers: <ViewerPanel client={clients.viewers} />,
    llm: <LlmPanel client={clients.llm} runtimeClient={clients.runtime} />,
    logs: <RuntimeLogsPanel client={clients.logs} launcherClient={clients.launcherLogs} adminClient={clients.auth} desktopLogs={desktop?.runtime_logs} />,
  };
  useEffect(() => {
    const report = (code: "browser_error" | "unhandled_rejection") => { void clients.logs.report({ code }); };
    const onError = () => report("browser_error");
    const onRejection = () => report("unhandled_rejection");
    const onRequestFailed = () => { void clients.logs.report({ code: "request_failed" }); };
    window.addEventListener("error", onError); window.addEventListener("unhandledrejection", onRejection); window.addEventListener("meowlive:request-failed", onRequestFailed);
    return () => { window.removeEventListener("error", onError); window.removeEventListener("unhandledrejection", onRejection); window.removeEventListener("meowlive:request-failed", onRequestFailed); };
  }, [clients.logs]);
  async function setServiceEnabled(id: LauncherServiceId, enabled: boolean) {
    if (serviceAction.current) return;
    serviceAction.current = true;
    setServiceBusy(true);
    try {
      await setDesktopServiceEnabled(id, enabled);
      const status = await getDesktopStatus();
      if (!status) throw new Error("桌面状态不可用");
      setDesktop(status);
      setError(null);
    } catch (failure) {
      feedback.error("服务启停失败", failure);
    } finally {
      serviceAction.current = false;
      setServiceBusy(false);
      setAttempt(value => value + 1);
    }
  }
  const errorNotice = error && <div className="error-banner" role="alert">{error} <button onClick={() => setAttempt(value => value + 1)}>重试桌面连接</button></div>;
  const serverLabel = desktop?.server.stopped ? "主服务已停止" : desktop?.server.ready ? "主服务运行中" : desktop?.server.last_error ? "主服务未就绪" : "主服务启动中";
  if (desktop === undefined) return <main className="studio-initial"><h1>MeowLive2D</h1>{errorNotice || <p role="status">正在读取桌面配置…</p>}</main>;
  if (desktop === null) return <ManagedWorkspace pages={panels} adminClient={clients.auth} allowManualService={import.meta.env.VITE_MEOWLIVE_LAUNCHER !== "true"} />;
  return <Workspace pages={panels} adminClient={clients.auth} setup={<><DesktopEnvironmentPanel platform={desktop.platform ?? "windows"} onBusyChanged={setEnvironmentBusy} onApplied={() => setAttempt(value => value + 1)} /><AppUpdatePanel disabled={environmentBusy || serviceBusy} /></>} ready={!error && desktop.server.ready} status={[{ label: serverLabel, available: !error && desktop.server.ready }, ...(desktop.platform === "linux" ? [] : [{ label: desktop.runtime.running ? "桌面执行端运行中" : "桌面执行端已停止", available: desktop.runtime.running }])]}
    notice={errorNotice} overview={<>{errorNotice}<DesktopLauncherControls status={desktop} busy={serviceBusy || environmentBusy} stale={!!error} setEnabled={setServiceEnabled} refresh={() => setAttempt(value => value + 1)} /></>} />;
}
