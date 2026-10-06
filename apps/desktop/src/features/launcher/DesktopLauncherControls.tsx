import type { LauncherService, LauncherServiceId, LauncherServiceState } from "@meowlive/contracts";
import { useState } from "react";
import type { DesktopStatus } from "../../services/desktop";
import { LauncherServiceCards } from "./LauncherControls";
import { openControlPanel } from "../../services/desktop";

export function DesktopLauncherControls({ status, busy, stale, setEnabled, refresh }: {
  status: DesktopStatus; busy: boolean; stale: boolean;
  setEnabled: (id: LauncherServiceId, enabled: boolean) => Promise<void>;
  refresh: () => void;
}) {
  const [browserError, setBrowserError] = useState(false);
  const { server, runtime, environment } = status;
  const isLinux = status.platform === "linux";
  const serverState: LauncherServiceState = server.stopped ? "stopped" : server.ready ? (server.managed ? "running" : "external") : server.last_error ? "failed" : "starting";
  const ttsReady = !!environment?.backend.ready && environment.models.some(model => model.id === "gpt-sovits-v2" && model.selected && model.downloaded);
  const service = (id: LauncherServiceId, state: LauncherServiceState, message: string, can_start: boolean, can_stop: boolean): LauncherService =>
    ({ id, state, message, can_start, can_stop, managed: state !== "external", url: "", log_path: "" });
  const items: LauncherService[] = [
    service("server", serverState, server.last_error ?? (server.stopped ? "打开开关启动主服务。" : "主服务运行中。关闭 App 时会自动释放。"), serverState === "stopped" || serverState === "failed", server.ready && server.managed),
    service("tts", environment?.inferenceRunning ? "running" : environment?.phase === "failed" ? "failed" : "stopped",
      !environment ? "正在等待环境状态，请刷新或重新打开 App。" : environment.busy || environment.phase === "failed" ? environment.message : !ttsReady ? "请先在环境与模型页安装、下载并选用 GPT-SoVITS v2。" : "打开开关启动语音引擎，关闭 App 时会自动释放。",
      ttsReady && server.ready, !!environment?.inferenceRunning),
    service("windows", runtime.running ? "running" : runtime.last_error ? "failed" : "stopped",
      isLinux ? "Windows 专属执行端，请在 Windows App 中使用。" : runtime.last_error ?? (runtime.simulation ? "静音模拟：不输出设备声音" : "系统音频输出"), !isLinux && server.ready, !isLinux && runtime.running),
  ];
  return <section className="panel launcher-panel" aria-labelledby="launcher-heading">
    <div className="section-title"><div><h2 id="launcher-heading">启动与运行</h2></div>
      <button onClick={refresh} disabled={busy}>刷新服务状态</button></div>
    <LauncherServiceCards windowsStatusMode="runtime" items={items} busy={busy || !!environment?.busy} stale={stale} setEnabled={setEnabled} />
    <div className="browser-panel-entry">
      <div><strong>浏览器控制面板</strong>{status.browser_panel_error ? <p role="alert">{status.browser_panel_error} App 仍可正常使用，处理后重新打开 App 即可重试。</p> : <p>保持此 App 运行即可在浏览器访问 <code>http://127.0.0.1:1420</code></p>}{browserError && <p role="alert">无法打开浏览器，请复制地址手动访问。</p>}</div>
      <button disabled={!!status.browser_panel_error} onClick={() => void openControlPanel().then(() => setBrowserError(false), () => setBrowserError(true))}>在浏览器中打开</button>
    </div>
    {!isLinux && <p className="field-hint">语音引擎与模型在 <a href="#setup">环境与模型</a> 中配置；大模型在 <a href="#llm">LLM 接入</a> 中配置。</p>}
    <details className="launcher-help"><summary>本机配置位置</summary>
      <p>桌面配置：<code>{status.config_path}</code></p>
      <p>主服务日志：<code>{server.log_path}</code></p>
    </details>
  </section>;
}
