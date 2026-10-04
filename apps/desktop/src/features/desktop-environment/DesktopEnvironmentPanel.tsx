import { useEffect, useRef, useState } from "react";
import { createEnvironmentClient, type EnvironmentClient, type EnvironmentRequest, type EnvironmentSnapshot } from "../../services/desktop/environment";

const capabilities: Record<string, string> = { training_inference: "训练与播报", transcription: "训练文本识别", download_only: "仅下载 · 尚未适配" };
export function DesktopEnvironmentPanel({ client, onApplied, onBusyChanged }: { client?: EnvironmentClient; onApplied?: () => void; onBusyChanged?: (busy: boolean) => void }) {
  const [defaultClient] = useState(createEnvironmentClient);
  const api = client ?? defaultClient;
  const [snapshot, setSnapshot] = useState<EnvironmentSnapshot | null>(null);
  const [distro, setDistro] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const mounted = useRef(false);
  const generation = useRef(0);
  const actionPending = useRef(false);
  useEffect(() => {
    mounted.current = true;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      const sequence = generation.current;
      try {
        const next = await api.status();
        if (!stopped && sequence === generation.current && !actionPending.current) {
          setSnapshot(next);
          setDistro(previous => previous && next.distros.some(item => item.name === previous && item.version === 2) ? previous : next.selectedDistro ?? next.distros.find(item => item.version === 2)?.name ?? "");
        }
      } catch (failure) { if (!stopped && sequence === generation.current) setError(failure instanceof Error ? failure.message : String(failure)); }
      if (!stopped) timer = setTimeout(() => void poll(), 2000);
    }
    void poll();
    return () => { stopped = true; mounted.current = false; clearTimeout(timer); };
  }, [api]);
  const busy = pending || snapshot?.busy === true;
  useEffect(() => { onBusyChanged?.(busy); }, [busy, onBusyChanged]);
  const backendReady = snapshot?.backend.ready === true && distro === snapshot.selectedDistro;
  const ttsSelected = snapshot?.models.some(model => model.selected && model.downloaded && ["training_inference"].includes(model.capability)) === true;
  async function run(action: EnvironmentRequest["action"], modelId?: string) {
    if (actionPending.current) return;
    actionPending.current = true; generation.current += 1; setPending(true); setError(null); setMessage(null);
    try {
      const next = await api.action({ action, ...(distro && action !== "install_wsl" && action !== "cancel" ? { distro } : {}), ...(modelId ? { modelId } : {}) });
      if (mounted.current) setSnapshot(next);
    } catch (failure) { if (mounted.current) setError(failure instanceof Error ? failure.message : String(failure)); }
    finally { actionPending.current = false; if (mounted.current) setPending(false); }
  }
  async function apply() {
    if (actionPending.current) return;
    actionPending.current = true; generation.current += 1; setPending(true); setError(null); setMessage(null);
    try { await api.apply(); if (mounted.current) { setMessage("训练后端已连接，主服务正在重新启动。就绪后可前往声音训练。"); onApplied?.(); } }
    catch (failure) { if (mounted.current) setError(failure instanceof Error ? failure.message : String(failure)); }
    finally { actionPending.current = false; if (mounted.current) setPending(false); }
  }
  return <div className="model-library">
    <section className="panel model-environment" aria-label="声音环境安装">
      <div className="model-environment-header"><h2>声音环境</h2><button disabled={busy} onClick={() => void run("detect")}>重新检测</button></div>
      <p className="muted">检测已有 WSL2 和语音后端，补齐环境后选择模型，即可在 App 内训练和播报。</p>
      {error && <p role="alert" className="error-banner">{error}</p>}
      {message && <p role="status">{message}</p>}
      {!snapshot ? <p role="status">正在读取环境状态…</p> : <>
        <p role="status">{snapshot.message}</p>
        <label>训练环境<select value={distro} disabled={busy} onChange={event => { setDistro(event.target.value); setMessage(null); }}>
          <option value="">请选择 WSL2 发行版</option>{snapshot.distros.map(item => <option key={item.name} value={item.name} disabled={item.version !== 2}>{item.name} · {item.version === 2 ? "WSL2" : "WSL1（需要升级）"}</option>)}
        </select></label>
        <p>{snapshot.backend.detail}</p>
        {snapshot.backend.gpu && <p className="field-hint">计算设备：已检测到可用 GPU</p>}
        <div className="model-card-actions">
          <button disabled={busy} onClick={() => void run("install_wsl")}>安装 WSL2</button>
          <button disabled={busy || !distro} onClick={() => void run("install_backend")}>安装或修复训练后端</button>
          <button className="primary-button" disabled={busy || !backendReady || !ttsSelected} onClick={() => void apply()}>连接训练后端</button>
        </div>
        <p className="field-hint">安装 WSL 可能请求管理员授权和重启。重启后打开 App，重新检测即可继续。已有环境不会被删除。</p>
        {snapshot.busy && <><progress aria-label="环境任务进度" max={100} value={snapshot.progress || undefined} /><button disabled={pending} onClick={() => void run("cancel")}>取消当前任务</button></>}
        <div className="model-card-actions"><button disabled={busy || !backendReady || !ttsSelected || snapshot.inferenceRunning} onClick={() => void run("start_inference")}>启动语音引擎</button><button disabled={busy || !snapshot.inferenceRunning} onClick={() => void run("stop_inference")}>停止语音引擎</button></div>
        <p className="field-hint">语音引擎{snapshot.inferenceRunning ? "运行中" : "未启动"}。训练前在声音训练页关闭语音模型，释放显存。</p>
        <details><summary>环境检测与安装日志</summary><pre className="model-path">{snapshot.logs.join("\n") || "暂无日志"}</pre></details>
      </>}
    </section>
    {snapshot && <section className="panel" aria-label="声音模型选择"><h2>选择声音模型</h2><p className="muted">下载后选用，再点击“连接训练后端”应用。声音生成和文本识别分别选择；切换模型前先停止语音引擎。</p>
      <div className="model-catalog-grid">{snapshot.models.map(model => <article className="model-catalog-card" key={model.id}>
        <h3>{model.name}</h3><p>{capabilities[model.capability] ?? model.capability}</p><p>{model.selected ? "已选用" : model.downloaded ? "已下载" : "尚未下载"}</p>
        <div className="model-card-actions"><button disabled={busy || !backendReady || model.downloaded} onClick={() => void run("download_model", model.id)} aria-label={`下载 ${model.name}`}>下载模型</button>
          <button disabled={busy || !backendReady || !model.downloaded || model.selected || snapshot.inferenceRunning || model.capability === "download_only"} onClick={() => void run("select_model", model.id)} aria-label={`选用 ${model.name}`}>{model.selected ? "已选用" : "选用模型"}</button></div>
      </article>)}</div>
      {snapshot.models.length === 0 && <p>检测或安装后端后显示可选模型。</p>}
    </section>}
  </div>;
}
