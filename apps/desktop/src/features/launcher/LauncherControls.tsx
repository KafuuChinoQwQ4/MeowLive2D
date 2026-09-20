import type { LauncherServiceId, LauncherServiceState } from "@meowlive/contracts";
import type { useLauncher } from "./useLauncher";

const labels: Record<LauncherServiceState, string> = {
  stopped: "未启动", starting: "启动中", running: "运行中", stopping: "停止中", failed: "启动失败", external: "外部运行中",
};
const windowsLabels: Record<LauncherServiceState, string> = {
  stopped: "未连接", starting: "连接中", running: "已连接", stopping: "断开中", failed: "连接失败", external: "其他执行端已连接",
};
export function launcherStateLabel(id: LauncherServiceId, state: LauncherServiceState) {
  return (id === "windows" ? windowsLabels : labels)[state];
}
const services: { id: LauncherServiceId; title: string; description: string }[] = [
  { id: "server", title: "主服务", description: "角色与互动" },
  { id: "tts", title: "TTS 语音引擎", description: "文字转语音" },
  { id: "windows", title: "Windows 执行端", description: "声音与口型" },
];

export function LauncherControls({ controller }: { controller: ReturnType<typeof useLauncher> }) {
  const { snapshot, error, stale, busy, setEnabled, refresh } = controller;
  return <section className="panel launcher-panel" aria-labelledby="launcher-heading">
      <div className="section-title"><div><h2 id="launcher-heading">启动与运行</h2></div>
        <button onClick={refresh} disabled={busy}>刷新服务状态</button></div>
      <p className="muted">启动顺序：主服务 → TTS → Windows 执行端。</p>
      <div className="launcher-services">
        {services.map(({ id, title, description }) => {
          const service = snapshot?.services.find(value => value.id === id);
          const on = !!service && ["starting", "running", "stopping", "external"].includes(service.state);
          return <article className="launcher-service" key={id}>
            <div className="launcher-service-heading"><div><h3>{title}</h3><p>{description}</p></div>
              <button type="button" role="switch" aria-label={title} aria-checked={on} className="service-switch"
                disabled={busy || stale || !service || service.state === "external" || (on ? !service.can_stop : !service.can_start)}
                onClick={() => void setEnabled(id, !on)}><span /></button></div>
            <div className="launcher-state-line"><span className={`task-status task-${service?.state ?? "unknown"}`}>{stale || !service ? "状态待确认" : launcherStateLabel(id, service.state)}</span>
              <span className="server-address">{service?.url}</span></div>
            <p className="launcher-message" role={service?.state === "failed" ? "alert" : undefined}>{stale ? "正在等待启动管理响应…" : service?.message}</p>
            {service?.state === "external" && <p className="field-hint">{id === "windows" ? "此连接由其他设备管理，请先在原设备退出。" : "请回其他终端停止该服务，再由此开关启动。"}</p>}
            {service && <details className="launcher-log"><summary>查看日志位置</summary><code>{service.log_path}</code></details>}
          </article>;
        })}
      </div>
      {error && <div className="error-banner" role="alert">{error} 请保持 Linux 中运行 <code>./launchers/start.sh</code> 的终端打开。</div>}
      {snapshot && <>
        <p className={snapshot.setup.llm_configured ? "field-hint" : "availability-note"}>{snapshot.setup.llm_message} <a href="#llm">前往 LLM 接入</a> · 保存后重启主服务生效。</p>
        <details className="launcher-help"><summary>本机配置位置</summary>
          <p>Windows 执行端：<code>{snapshot.setup.windows_client_path}</code></p>
          <p>引擎路径配置：<code>{snapshot.setup.configuration_path}</code></p>
          <p>语音与模型：<code>{snapshot.setup.server_config}</code></p><p>修改文件后重启控制面板。</p>
        </details>
      </>}
    </section>;
}
