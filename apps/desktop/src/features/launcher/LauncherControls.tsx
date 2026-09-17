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
  { id: "server", title: "主服务", description: "管理角色、播报任务和 Agent 互动。" },
  { id: "tts", title: "TTS 语音引擎", description: "提供文字转语音服务，模型在“训练与离线”中单独启用或关闭。" },
  { id: "windows", title: "Windows 执行端", description: "连接 Windows 播放声音，并启用 VTube Studio 连接与口型。" },
];

export function LauncherControls({ controller }: { controller: ReturnType<typeof useLauncher> }) {
  const { snapshot, error, stale, busy, setEnabled, refresh } = controller;
  return <section className="panel launcher-panel" aria-labelledby="launcher-heading">
      <div className="section-title"><div><p className="eyebrow">从这里开始</p><h2 id="launcher-heading">启动与运行</h2></div>
        <button onClick={refresh} disabled={busy}>刷新服务状态</button></div>
      <p className="field-hint">配置与日志路径以 <code>./</code> 为当前项目目录，<code>~/</code> 为运行控制面板的 Linux 用户主目录。</p>
      <p className="muted">依次打开主服务、TTS 和 Windows 执行端开关，等待服务就绪、执行端已连接。主服务关闭时，这个控制面板仍然保留。</p>
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
            {service?.state === "external" && <p className="field-hint">{id === "windows" ? "当前连接来自其他执行端，无法在此关闭；请先退出该执行端，再使用此开关连接。" : "请回其他终端停止该服务，再由此开关启动。"}</p>}
            {service && <details className="launcher-log"><summary>查看日志位置</summary><code>{service.log_path}</code></details>}
          </article>;
        })}
      </div>
      {error && <div className="error-banner" role="alert">{error} 请保持 Linux 中运行 <code>./launchers/start.sh</code> 的终端打开。</div>}
      <ol className="launcher-steps">
        <li><strong>开启主服务</strong><span>等到“运行中”，从导航进入需要的功能。</span></li>
        <li><strong>开启 TTS</strong><span>服务就绪后，到“训练与离线”点击“启用语音模型”；关闭模型可释放权重内存。</span></li>
        <li><strong>连接 Windows 执行端</strong><span>主服务就绪后打开第三个开关，等到“已连接”即可播放声音。</span></li>
        <li><strong>准备自己的音色</strong><span>到“角色与音色”上传参考录音并选择音色，即可试播。需要微调时再到“训练与离线”训练、试听并保存。</span></li>
      </ol>
      {snapshot && <>
        <p className={snapshot.setup.llm_configured ? "field-hint" : "availability-note"}>{snapshot.setup.llm_message} <a href="#llm">前往 LLM 接入</a>。这里显示的是主服务启动时读取的状态；保存后重启主服务才会刷新并生效。</p>
        <details className="launcher-help"><summary>首次使用、配置位置与退出方法</summary>
          <p>Windows 首次使用时，双击项目中的 <code>launchers/start-windows.cmd</code>，按提示完成环境准备。随后在此打开 Windows 执行端开关。</p>
          <p>Windows 程序和配置位于 <code>{snapshot.setup.windows_client_path}</code>。首次需自行构建执行程序并准备本机配置，完成后开关会启动并连接执行端。</p>
          <p>打开 Windows 执行端开关会自动保存并启用 VTube Studio 连接；请在 VTS 中开启插件 API，首次连接时允许授权。关闭开关会停止执行端，VTS 启用设置会保留。OBS 需按启动手册单独配置。</p>
          <p>引擎路径配置：<code>{snapshot.setup.configuration_path}</code></p>
          <p>语音与模型配置：<code>{snapshot.setup.server_config}</code>。修改文件后关闭并重开控制面板。</p>
          <p>关闭主服务会断开执行端并结束当前会话；关闭 TTS 会中断正在生成的语音。退出时先关闭 Windows 执行端，再关闭 TTS 和主服务开关，再在 Linux 终端按 Ctrl+C。直接 Ctrl+C 也会回收本次启动的服务。</p>
        </details>
      </>}
    </section>;
}
