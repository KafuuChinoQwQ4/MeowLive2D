import modelCatalog from "../../../../../scripts/voice-backend-models.json";
import { useEffect, useRef, useState } from "react";
import { createEnvironmentClient, type EnvironmentClient, type EnvironmentRequest, type EnvironmentSnapshot } from "../../services/desktop/environment";

const modelPurposes = new Map(modelCatalog.map(model => [model.id, model.purpose]));
const capabilities: Record<string, string> = { training_inference: "训练与播报", transcription: "训练文本识别", download_only: "仅下载 · 尚未适配" };
export function DesktopEnvironmentPanel({ client, onApplied, onBusyChanged, platform = "windows" }: { client?: EnvironmentClient; onApplied?: () => void; onBusyChanged?: (busy: boolean) => void; platform?: "windows" | "linux" }) {
  const [defaultClient] = useState(createEnvironmentClient);
  const api = client ?? defaultClient;
  const [snapshot, setSnapshot] = useState<EnvironmentSnapshot | null>(null);
  const [distro, setDistro] = useState("");
  const [configPath, setConfigPath] = useState("");
  const [modelSearch, setModelSearch] = useState("");
  const [modelCategory, setModelCategory] = useState<"all" | "asr" | "tts">("all");
  const [modelPage, setModelPage] = useState(0);
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
  const backendReady = snapshot?.backend.ready === true && (platform === "linux" || distro === snapshot.selectedDistro);
  const ttsModel = snapshot?.models.find(model => model.id === "gpt-sovits-v2");
  const ttsSelected = ttsModel?.selected === true && ttsModel.downloaded;
  const query = modelSearch.trim().toLowerCase();
  const filteredModels = snapshot?.models.filter(model => {
    const purpose = modelPurposes.get(model.id) ?? (model.capability === "transcription" ? "asr" : "tts");
    const categoryMatch = modelCategory === "all" || modelCategory === purpose;
    return categoryMatch && (!query || `${model.name} ${model.id}`.toLowerCase().includes(query));
  }) ?? [];
  const modelPageCount = Math.max(1, Math.ceil(filteredModels.length / 4));
  const currentModelPage = Math.min(modelPage, modelPageCount - 1);
  const visibleModels = filteredModels.slice(currentModelPage * 4, (currentModelPage + 1) * 4);
  const connectionHint = busy ? "正在处理，请稍候。"
    : !distro ? "请选择 WSL2 环境。"
    : snapshot?.selectedDistro !== distro ? "环境已切换，请重新检测。"
    : !backendReady ? "后端未就绪，请展开配置与修复。"
    : !ttsModel?.downloaded ? "请先下载下方的 GPT-SoVITS v2。"
    : !ttsSelected ? "模型已检测，尚未选用。"
    : "已就绪，可以连接。";
  async function run(action: EnvironmentRequest["action"], modelId?: string) {
    if (actionPending.current) return;
    actionPending.current = true; generation.current += 1; setPending(true); setError(null); setMessage(null);
    try {
      const next = await api.action({ action, ...(distro && action !== "install_wsl" && action !== "cancel" ? { distro } : {}), ...(modelId ? { modelId } : {}), ...(action === "detect" && configPath.trim() ? { configPath: configPath.trim() } : {}) });
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
  return <div className="model-library desktop-environment">
    <section className="panel model-environment" aria-label="声音环境安装">
      <div className="model-environment-header"><h2>声音环境</h2><button disabled={busy} onClick={() => void run("detect")}>重新检测</button></div>
      {error && <p role="alert" className="error-banner">{error}</p>}
      {message && <p role="status">{message}</p>}
      {!snapshot ? <p role="status">正在读取环境状态…</p> : <>
        {platform === "linux" ? <div className="linux-environment-summary">
          <div className="environment-status-row"><span className={`model-badge ${snapshot.backend.ready ? "is-ready" : "is-pending"}`}>{snapshot.backend.ready ? "本机后端已检测" : "未检测到本机后端"}</span>
            {snapshot.backend.ready && <span className="model-badge">{snapshot.backend.gpu ? "CUDA GPU" : "CPU"}</span>}
          </div>
          <p role="status" className="field-hint">{snapshot.backend.detail || "Linux App 可检测本机语音后端，并直接管理模型下载、选用和语音引擎启停。"}</p>
          {snapshot.backend.ready && <p className="field-hint model-path">{snapshot.backend.engineRoot}<br />Python：{snapshot.backend.pythonPath}</p>}
          <p className="muted">模型库与浏览器同步，Linux App 直接调用本机 voice-backend 控制器。</p>
        </div> : <>
        <div className="environment-status-row">
          <span className={`model-badge ${backendReady ? "is-ready" : "is-pending"}`}>{backendReady ? "后端就绪" : "待配置"}</span>
          {backendReady && <span className="model-badge">{snapshot.backend.gpu ? "CUDA GPU" : "CPU"}</span>}
          <span className="model-badge">{snapshot.inferenceRunning ? "语音引擎运行中" : "语音引擎未启动"}</span>
        </div>
        {(busy || snapshot.phase === "failed" || snapshot.phase === "reboot_required") && <p role="status" className="field-hint">{snapshot.message}</p>}
        {busy && <div className="environment-task">
          <div className={`environment-progress ${!pending && snapshot.progress > 0 && snapshot.phase !== "detecting" ? "is-determinate" : "is-indeterminate"}`}
            role="progressbar" aria-label="环境任务进度" aria-valuemin={0} aria-valuemax={100}
            aria-valuenow={!pending && snapshot.progress > 0 && snapshot.phase !== "detecting" ? snapshot.progress : undefined}>
            <span style={{ transform: `scaleX(${snapshot.progress / 100})` }} />
          </div>
          {snapshot.busy && <button disabled={pending} onClick={() => void run("cancel")}>取消当前任务</button>}
        </div>}
        <label>训练环境<select value={distro} disabled={busy} onChange={event => { setDistro(event.target.value); setMessage(null); }}>
          <option value="">请选择 WSL2 发行版</option>{snapshot.distros.map(item => <option key={item.name} value={item.name} disabled={item.version !== 2}>{item.name} · {item.version === 2 ? "WSL2" : "WSL1（需要升级）"}</option>)}
        </select></label>
        <div className="model-card-actions">
          <button className="primary-button" aria-describedby="environment-connection-hint" disabled={busy || !backendReady || !ttsSelected} onClick={() => void apply()}>连接训练后端</button>
        </div>
        <div className="environment-connection-help">
          <p id="environment-connection-hint" className="field-hint">{connectionHint}</p>
          {backendReady && ttsModel?.downloaded && !ttsSelected && <button disabled={busy || snapshot.inferenceRunning} onClick={() => void run("select_model", ttsModel.id)}>选用已检测模型</button>}
        </div>
        <div className="model-card-actions environment-engine-actions">
          {snapshot.inferenceRunning
            ? <button disabled={busy} onClick={() => void run("stop_inference")}>停止语音引擎</button>
            : <button disabled={busy || !backendReady || !ttsSelected} onClick={() => void run("start_inference")}>启动语音引擎</button>}
        </div>
        <details className="environment-advanced"><summary>环境配置与修复</summary>
          <div className="model-card-actions">
            <button disabled={busy} onClick={() => void run("install_wsl")}>安装 WSL2</button>
            <button disabled={busy || !distro} onClick={() => void run("install_backend")}>安装或修复训练后端</button>
          </div>
          <p className="field-hint">安装 WSL 可能需要管理员授权和重启；已有环境会保留。</p>
        <details><summary>使用已有训练环境</summary>
          <p className="field-hint">自定义安装位置：填写 WSL 配置路径后重新检测。</p>
          <label>WSL 配置文件路径<input value={configPath} disabled={busy} onChange={event => setConfigPath(event.target.value)} placeholder="例如 /home/me/MeowLive2D/config/server.local.toml" /></label>
        </details>
        <p>{snapshot.backend.detail}</p>
        {snapshot.backend.ready && <p className="field-hint model-path">已检测环境：{snapshot.selectedDistro} · {snapshot.backend.engineRoot}<br />Python：{snapshot.backend.pythonPath}</p>}
        </details>
        <details className="environment-log"><summary>环境检测与安装日志</summary><pre tabIndex={0} aria-label="环境日志内容">{snapshot.logs.join("\n") || "暂无日志"}</pre></details>
        </>}
      </>}
    </section>
    {snapshot && <section className="panel" aria-label="声音模型选择"><h2>选择声音模型</h2><p className="muted">选用模型后，点击连接训练后端。标注“仅下载”的模型尚未接入训练、播报或文本识别。</p>
      {snapshot.models.length > 0 && <div className="model-search"><div className="segmented-control" role="group" aria-label="模型分类">{([ ["all", "全部模型"], ["asr", "语音转文字"], ["tts", "训练语音与播报"] ] as const).map(([id, label]) => <button key={id} aria-pressed={modelCategory === id} onClick={() => { setModelCategory(id); setModelPage(0); }}>{label}</button>)}</div><label>搜索声音模型<input type="search" value={modelSearch} onChange={event => { setModelSearch(event.target.value); setModelPage(0); }} placeholder="按模型名称搜索" /></label></div>}
      <div className="model-catalog-grid">{visibleModels.map(model => <article className="model-catalog-card" key={model.id}>
        <h3>{model.name}</h3><p>{capabilities[model.capability] ?? model.capability}</p><p>{model.selected ? "已选用" : model.downloaded ? "已下载" : "尚未下载"}</p>
        <div className="model-card-actions"><button disabled={busy || !backendReady || model.downloaded} onClick={() => void run("download_model", model.id)} aria-label={`下载 ${model.name}`}>下载模型</button>
          <button disabled={busy || !backendReady || !model.downloaded || model.selected || snapshot.inferenceRunning || model.capability === "download_only"} onClick={() => void run("select_model", model.id)} aria-label={`选用 ${model.name}`}>{model.selected ? "已选用" : "选用模型"}</button></div>
      </article>)}</div>
      {snapshot.models.length === 0 && <p>检测或安装后端后显示可选模型。</p>}
      {snapshot.models.length > 0 && visibleModels.length === 0 && <p>没有匹配的模型。</p>}
      {filteredModels.length > 0 && <div className="model-pagination"><span>共 {filteredModels.length} 个模型 · 第 {currentModelPage + 1} / {modelPageCount} 页</span>{modelPageCount > 1 && <div><button disabled={currentModelPage === 0} onClick={() => setModelPage(page => page - 1)}>上一页</button><button disabled={currentModelPage + 1 >= modelPageCount} onClick={() => setModelPage(page => page + 1)}>下一页</button></div>}</div>}
    </section>}
  </div>;
}
