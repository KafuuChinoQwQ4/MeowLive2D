import { ModelLibraryManualGuide } from "../features/model-library/ModelLibraryPanel";
import { SpeechPanel } from "../features/live";
import { AgentPanel } from "../features/agent";
import { LlmPanel } from "../features/llm";
import { ConnectionPanel } from "../features/connections";
import { TrainingPanel } from "../features/training";
import { ResourcesPanel } from "./resources";
import { useEffect, useMemo, useState } from "react";
import { getDesktopStatus, type DesktopStatus } from "../services/desktop";
import { createServerClient } from "../services/server";
import { createAgentClient } from "../services/server/agent";
import { createLiveClient } from "../services/server/live";
import { createResourceClient } from "../services/server/resources";
import { createTrainingClient } from "../services/server/training";
import { createObsClient } from "../services/server/obs";
import { createLlmClient } from "../services/server/llm";
import { ObsPanel } from "../features/obs";
import { ManagedWorkspace } from "./ManagedWorkspace";
import { Workspace, type WorkspaceProps } from "./Workspace";
import { FeedbackProvider, useFeedback } from "./feedback/OperationFeedback";

export function App() {
  return <FeedbackProvider><AppContent /></FeedbackProvider>;
}

function AppContent() {
  const feedback = useFeedback();
  const [desktop, setDesktop] = useState<DesktopStatus | null | undefined>(undefined);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let cancelled = false;
    let retryRequested = attempt > 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const update = async () => {
      try {
        const status = await getDesktopStatus();
        if (cancelled) return;
        setDesktop(status);
        setError(null);
        feedback.clearIssue("desktop:configuration");
        if (status && !status.runtime.running) feedback.reportIssue("desktop:runtime", "桌面执行端已停止", status.runtime.last_error || "请关闭并重新启动桌面程序。");
        else feedback.clearIssue("desktop:runtime");
        if (retryRequested) { retryRequested = false; feedback.success("桌面连接已恢复", "已读取桌面配置。"); }
        if (status) timer = setTimeout(() => void update(), 3_000);
      } catch {
        if (!cancelled) {
          setError("桌面配置读取失败，请检查执行端状态后重试。");
          if (retryRequested) { retryRequested = false; feedback.error("重试桌面连接失败", "请检查执行端状态后重试。"); }
          else feedback.reportIssue("desktop:configuration", "桌面配置读取失败", "请检查执行端状态后重试。");
        }
      }
    };
    void update();
    return () => { cancelled = true; clearTimeout(timer); };
  }, [attempt, feedback]);
  const baseUrl = desktop?.server_url;
  const clients = useMemo(() => ({
    speech: createServerClient({ baseUrl }), agent: createAgentClient({ baseUrl }),
    live: createLiveClient({ baseUrl }), resources: createResourceClient({ baseUrl }),
    training: createTrainingClient({ baseUrl }), obs: createObsClient({ baseUrl }),
    llm: createLlmClient({ baseUrl }),
  }), [baseUrl]);
  const panels: WorkspaceProps["pages"] = {
    live: <ConnectionPanel client={clients.live} />,
    obs: <ObsPanel client={clients.obs} />,
    resources: <ResourcesPanel resourceClient={clients.resources} speechClient={clients.speech} />,
    training: <TrainingPanel client={clients.training} resources={clients.resources} />,
    speech: <SpeechPanel client={clients.speech} />,
    agent: <AgentPanel client={clients.agent} />,
    llm: <LlmPanel client={clients.llm} />,
  };
  const errorNotice = error && <div className="error-banner" role="alert">{error} <button onClick={() => setAttempt(value => value + 1)}>重试桌面连接</button></div>;
  if (desktop === undefined) return <main className="studio-initial"><h1>MeowLive2D</h1>{errorNotice || <p role="status">正在读取桌面配置…</p>}</main>;
  if (desktop === null && import.meta.env.VITE_MEOWLIVE_LAUNCHER === "true") return <ManagedWorkspace pages={panels} />;
  return <Workspace pages={panels} setup={<ModelLibraryManualGuide />} ready status={[{ label: desktop ? (desktop.runtime.running ? "桌面执行端运行中" : "桌面执行端已停止") : "手动服务模式", available: desktop?.runtime.running ?? null }]}
    notice={errorNotice} overview={<>{errorNotice}{desktop ? <section className="connection-card" aria-label="桌面执行端">
      <div><h2>{desktop.runtime.running ? "桌面执行端运行中" : "桌面执行端已停止"}</h2><p className="server-address">{desktop.server_url}</p><p className="muted">{desktop.runtime.simulation ? "静音模拟：不输出设备声音" : "系统音频输出"}</p></div>
      {!desktop.runtime.running && <p role="alert">执行端已停止，请关闭并重新启动桌面程序。{desktop.runtime.last_error}</p>}
    </section> : <section className="panel studio-manual"><h2>从导航开始</h2><p className="muted">当前为手动服务模式。从左侧选择功能，即可进入对应的工作区。</p><p className="availability-note">需要在网页启停主服务、TTS 和 Windows 执行端时，在项目目录使用 <code>./launchers/start.sh</code> 启动控制面板。</p></section>}</>} />;
}
