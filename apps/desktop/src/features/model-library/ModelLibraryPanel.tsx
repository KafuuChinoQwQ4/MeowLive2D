import { useId, useState } from "react";
import type { ModelLibraryClient } from "../../services/model-library";
import { useModelLibrary } from "./useModelLibrary";
import { InstalledModels } from "./InstalledModels";

const stateLabels: Record<string, string> = { queued: "等待下载", downloading: "下载中", completed: "下载完成", failed: "下载失败", cancelled: "已取消" };
const formatBytes = (value: number) => value >= 1073741824 ? `${(value / 1073741824).toFixed(1)} GB` : `${(value / 1048576).toFixed(1)} MB`;
export function ModelLibraryManualGuide() {
  return <section className="panel model-manual-guide"><h2>打开模型管理</h2><p className="muted">在 Linux / WSL 项目目录运行 <code>./launchers/start.sh</code>。</p><p className="muted">然后打开 <a href="http://127.0.0.1:1420">控制面板</a>，进入“环境与模型”。</p><a href="#guide">查看环境准备步骤</a></section>;
}
export function ModelLibraryPanel({ client, token, onSelected }: { client: ModelLibraryClient; token: string | null; onSelected: () => void }) {
  const { snapshot, error, stale, busy, run, refresh } = useModelLibrary(client, token, onSelected);
  const [tab, setTab] = useState<"installed" | "catalog" | "downloads">("installed");
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("all");
  const [purpose, setPurpose] = useState("all");
  const [pages, setPages] = useState<Record<string, number>>({});
  const tabId = useId();
  const disabled = !token || stale || busy || !snapshot?.environment.ready;
  const models = snapshot?.catalog.filter(model => `${model.name} ${model.languages} ${model.description}`.toLowerCase().includes(query.toLowerCase()) && (filter === "all" || model.compatibility === filter || model.purpose === filter)) ?? [];
  const activeDownloads = snapshot?.downloads.filter(download => ["queued", "downloading"].includes(download.state)) ?? [];
  const chooseTab = (next: typeof tab) => setTab(next);
  return <div className="model-library">
    {error && <div className="error-banner" role="alert">{error} <button onClick={refresh}>刷新状态</button></div>}
    {!snapshot ? <section className="panel"><p role="status">正在检查运行环境和本地模型…</p></section> : <>
      <section className="panel model-environment" aria-label="环境检查">
        <div className="model-environment-header"><div><h2>运行环境</h2></div><span className={`model-badge ${snapshot.environment.ready ? "is-ready" : "is-pending"}`}>{({ wsl2: "WSL2", wsl1: "WSL1 · 需要升级", linux: "原生 Linux", unknown: "环境待确认" })[snapshot.environment.kind]}</span></div>
        <p className="muted">{snapshot.environment.message}</p>
        <div className="model-environment-details"><span><strong>运行环境</strong>{snapshot.environment.distro || "Linux"}</span><span><strong>推理引擎</strong>{snapshot.runtime.ready ? "GPT-SoVITS 已就绪" : "尚需配置"}</span><span><strong>本地模型</strong>{snapshot.installed.length} 个已发现</span></div>
        {!snapshot.environment.ready && <div className="availability-note"><strong>请先完成 Windows 环境准备</strong><p>运行 <code>launchers/start-windows.cmd</code>，完成 WSL2 准备。</p><a href="https://learn.microsoft.com/zh-cn/windows/wsl/install" target="_blank" rel="noreferrer">微软安装与升级指南 ↗</a></div>}
        {!snapshot.runtime.ready && <div className="availability-note"><strong>还需要准备语音引擎</strong><p>{snapshot.runtime.message}</p><p>按安装说明准备引擎，在 <code>config/local/launcher.json</code> 配置路径后重启。</p><a href="https://github.com/RVC-Boss/GPT-SoVITS" target="_blank" rel="noreferrer">GPT-SoVITS 安装说明 ↗</a></div>}
        <details className="model-system-details"><summary>查看检测详情</summary><p>内核：<code>{snapshot.environment.release}</code></p><p>引擎：<code>{snapshot.runtime.engine_root}</code></p><p>Python：<code>{snapshot.runtime.python_path}</code></p><p>扫描范围：</p><ul>{snapshot.scan_roots.map(path => <li key={path}><code>{path}</code></li>)}</ul><p>安装后点击“重新扫描”。</p></details>
      </section>
      <section className="panel model-browser">
        {tab === "installed" && <div className="model-search model-purpose-filter"><label>模型用途<select value={purpose} onChange={event => { setPurpose(event.target.value); setPages({}); }}><option value="all">全部用途</option><option value="tts">声音生成 · 播报与训练</option><option value="asr">语音识别 · 训练文本提取</option></select></label></div>}
        <div className="model-tabs" role="tablist" aria-label="模型管理分类">{([ ["installed", `本地模型 · ${snapshot.installed.length}`], ["catalog", `下载模型 · ${snapshot.catalog.length}`], ["downloads", `下载任务${activeDownloads.length ? ` · ${activeDownloads.length}` : ""}`] ] as const).map(([id, label]) => <button key={id} id={`${tabId}-${id}`} role="tab" aria-selected={tab === id} aria-controls={`${tabId}-${id}-panel`} tabIndex={tab === id ? 0 : -1} onClick={() => chooseTab(id)} onKeyDown={event => { if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) { event.preventDefault(); const tabs = ["installed", "catalog", "downloads"] as const; const index = event.key === "Home" ? 0 : event.key === "End" ? 2 : (tabs.indexOf(id) + (event.key === "ArrowRight" ? 1 : 2)) % 3; chooseTab(tabs[index]); document.getElementById(`${tabId}-${tabs[index]}`)?.focus(); } }}>{label}</button>)}</div>
        <div id={`${tabId}-installed-panel`} role="tabpanel" aria-labelledby={`${tabId}-installed`} hidden={tab !== "installed"}>
          <div className="model-section-heading"><div><h2>选择本地语音模型</h2><p className="muted">声音生成与语音识别分别选择，识别模型帮助自动填写训练文本。</p></div><button disabled={disabled} onClick={() => void run("scan")}>{busy ? "处理中…" : "重新扫描"}</button></div>
          {snapshot.asr_error && <p className="error-banner" role="alert">{snapshot.asr_error}</p>}
          <InstalledModels models={snapshot.installed} purpose={purpose} disabled={disabled} select={id => { void run("select", id); }}
            browse={kind => { setPurpose(kind); setQuery(""); setFilter(kind); setPages({}); chooseTab("catalog"); }} />
        </div>
        <div id={`${tabId}-catalog-panel`} role="tabpanel" aria-labelledby={`${tabId}-catalog`} hidden={tab !== "catalog"}>
          <div className="model-section-heading"><div><h2>模型库</h2><p className="muted">“已接入”可直接使用；“需适配”仅供下载。</p></div></div>
          <div className="model-search"><label>搜索语音模型<input type="search" placeholder="模型名称、语言或用途" value={query} onChange={event => { setQuery(event.target.value); setPages({}); }} /></label><label>使用范围<select className="model-scope-select" value={filter} onChange={event => { setFilter(event.target.value); setPages({}); }}><option value="all">全部模型</option><optgroup label="按模型用途"><option value="tts">音色训练与声音生成</option><option value="asr">语音转文本</option></optgroup><optgroup label="按接入状态"><option value="ready">本项目已接入</option><option value="download_only">可下载 · 需适配</option></optgroup></select></label></div>
          <div className="model-catalog-groups" role="region" aria-label="可下载模型">{(["tts", "asr"] as const).map(kind => {
            const items = models.filter(model => model.purpose === kind);
            if (!items.length) return null;
            const pageCount = Math.max(1, Math.ceil(items.length / 4));
            const currentPage = Math.min(pages[kind] ?? 0, pageCount - 1);
            return <section className="model-catalog-group" key={kind} aria-label={kind === "tts" ? "音色训练与声音生成" : "语音转文本"}>
              <div className="model-category-heading"><div><h3>{kind === "tts" ? "音色训练与声音生成" : "语音转文本"}</h3><p>{kind === "tts" ? "克隆音色、训练声音，用于语音播报。" : "识别本地录音，自动填写参考文本与训练文本。"}</p></div><span className="model-badge">{items.length} 个模型</span></div>
              <div className="model-catalog-grid">{items.slice(currentPage * 4, (currentPage + 1) * 4).map(model => {
            const downloading = activeDownloads.some(download => download.model_id === model.id);
            return <article className="model-catalog-card" key={model.id}><div className="model-card-title"><h3>{model.name}</h3><span className={`model-badge ${model.compatibility === "ready" ? "is-ready" : "is-pending"}`}>{model.compatibility === "ready" ? "已接入" : "需适配"}</span></div><p className="model-language">{model.purpose === "asr" ? "语音识别" : "声音生成"} · {model.languages}</p><p className="model-description">{model.description}</p><p className="model-compatibility">{model.note}</p><p className="model-license">许可：{model.license}</p><div className="model-card-actions"><button className={model.compatibility === "ready" ? "primary-button" : ""} disabled={disabled || downloading} onClick={() => { void run("download", model.id); chooseTab("downloads"); }}>{downloading ? "下载进行中" : "下载权重"}</button><a href={model.homepage} target="_blank" rel="noreferrer">官方说明 ↗</a><a href={model.source_url} target="_blank" rel="noreferrer">权重来源 ↗</a></div></article>;
              })}</div>
              <div className="model-pagination"><span>共 {items.length} 个模型 · 第 {currentPage + 1} / {pageCount} 页</span>{pageCount > 1 && <div><button disabled={currentPage === 0} onClick={() => setPages(previous => ({ ...previous, [kind]: currentPage - 1 }))}>上一页</button><button disabled={currentPage + 1 >= pageCount} onClick={() => setPages(previous => ({ ...previous, [kind]: currentPage + 1 }))}>下一页</button></div>}</div>
            </section>;
          })}</div>
          {!models.length && <p className="model-empty">没有匹配的模型，试试其他名称或语言。</p>}
        </div>
        <div id={`${tabId}-downloads-panel`} role="tabpanel" aria-labelledby={`${tabId}-downloads`} hidden={tab !== "downloads"}>
          <div className="model-section-heading"><div><h2>下载任务</h2><p className="muted">保持启动终端打开，下载后重新扫描。</p></div></div>
          {snapshot.downloads.length ? <div className="model-download-list">{snapshot.downloads.map(job => <article className="model-download-card" key={job.id}><div className="model-card-title"><h3>{snapshot.catalog.find(model => model.id === job.model_id)?.name ?? job.model_id}</h3><span className="model-badge">{stateLabels[job.state]}</span></div><p>{job.message}</p>{["queued", "downloading"].includes(job.state) && <progress aria-label={`${job.model_id} 下载进度`} max={job.total_bytes || undefined} value={job.total_bytes ? Math.min(job.downloaded_bytes, job.total_bytes) : undefined} />}<div className="model-download-meta"><span>{formatBytes(job.downloaded_bytes)}{job.total_bytes ? ` / ${formatBytes(job.total_bytes)}` : " · 总大小待确认"}</span>{["queued", "downloading"].includes(job.state) && <button disabled={disabled} onClick={() => void run("cancel", job.id)}>取消下载</button>}{["failed", "cancelled"].includes(job.state) && <button disabled={disabled || activeDownloads.some(active => active.model_id === job.model_id)} onClick={() => void run("download", job.model_id)}>重试下载</button>}</div><code className="model-path">{job.path}</code></article>)}</div> : <div className="model-empty"><span aria-hidden="true">↓</span><h3>还没有下载任务</h3><p>前往模型库开始下载。</p><button onClick={() => chooseTab("catalog")}>前往模型库</button></div>}
        </div>
      </section>
    </>}
  </div>;
}
