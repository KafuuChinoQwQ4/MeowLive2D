import type { TrainingPerformance } from "@meowlive/contracts";
import { useEffect, useRef, useState } from "react";
import { AUDIO_FILE_ACCEPT, MAX_AUDIO_SOURCE_BYTES, prepareAudioFile } from "../../services/audio";
import { createTrainingClient, type TrainingClient } from "../../services/server/training";
import { createResourceClient, type ResourcesClient } from "../../services/server/resources";
import { loadTrainingPreferences, saveTrainingPreferences, type TrainingPerformancePreset } from "../../services/trainingPreferences";
import { useTraining } from "./useTraining";
import { TrainingResultDialog } from "./TrainingResultDialog";
import { terminalFeedback } from "./trainingFeedback";
import { groupTrainingVoices, TrainingVoiceLibrary } from "./TrainingVoiceLibrary";
import { useFeedback } from "../../app/feedback/OperationFeedback";
const defaultClient = createTrainingClient();
const defaultResources = createResourceClient();
const MAX_TRAINING_BYTES = 32 * 1024 * 1024;
const phases: Record<string, string> = { queued: "排队", preparing: "准备素材", training: "训练中", validating: "校验权重", completed: "训练完成", failed: "训练失败", cancelling: "取消中", cancelled: "已取消", interrupted: "已中断" };

const PAGE_SIZE = 5;
const tabs = [["create", "新建训练"], ["history", "训练记录"], ["voices", "已训练音色"], ["offline", "离线设置"]] as const;
type TrainingTab = typeof tabs[number][0];
const performancePresets = {
  low: { batch_size: 1, data_workers: 1, cpu_threads: 2 },
  balanced: { batch_size: 2, data_workers: 2, cpu_threads: 4 },
  high: { batch_size: 4, data_workers: 4, cpu_threads: 8 },
};
const bounded = (value: string, min: number, max: number) => Math.min(max, Math.max(min, Math.trunc(Number(value)) || min));
function Pagination({ label, count, page, onPage }: { label: string; count: number; page: number; onPage: (page: number) => void }) {
  const pages = Math.max(1, Math.ceil(count / PAGE_SIZE));
  return <nav className="training-pagination" aria-label={`${label}分页`}>
    <span>共 {count} 条 · 第 {page + 1} / {pages} 页</span>
    <button type="button" aria-label={`${label}上一页`} disabled={page === 0} onClick={() => onPage(page - 1)}>上一页</button>
    <button type="button" aria-label={`${label}下一页`} disabled={page + 1 >= pages} onClick={() => onPage(page + 1)}>下一页</button>
  </nav>;
}

export function TrainingPanel({ client = defaultClient, resources = defaultResources }: { client?: TrainingClient; resources?: ResourcesClient }) {
  return <TrainingPanelContent key={resources.baseUrl} client={client} resources={resources} />;
}

function TrainingPanelContent({ client, resources }: { client: TrainingClient; resources: ResourcesClient }) {
  const c = useTraining(client, resources);
  const feedback = useFeedback();
  const [initialPreferences] = useState(() => loadTrainingPreferences(resources.baseUrl));
  const [tab, setTab] = useState<TrainingTab>("create");
  const [clipPage, setClipPage] = useState(0);
  const [clipsExpanded, setClipsExpanded] = useState(true);
  const [jobPage, setJobPage] = useState(0);
  const [performancePreset, setPerformancePreset] = useState(initialPreferences.performancePreset);
  const [performance, setPerformance] = useState<TrainingPerformance>(initialPreferences.performance);
  const jobs = c.snapshot?.jobs.slice().reverse() ?? [];
  const versions = c.snapshot?.versions ?? [];
  const currentJobPage = Math.min(jobPage, Math.max(0, Math.ceil(jobs.length / PAGE_SIZE) - 1));
  const voiceGroups = groupTrainingVoices(versions, c.voices);
  const runtime = c.modelRuntime;
  const modelsLoaded = runtime?.state === "loaded";
  const modelsTransitioning = runtime?.state === "loading" || runtime?.state === "unloading";
  const canSynthesize = runtime?.state === "loaded" || runtime?.state === "unsupported";
  const modelSwitchDisabled = c.pending || !!c.snapshot?.busy || !runtime?.supported || modelsTransitioning || runtime.state === "unavailable";

  const [name, setName] = useState("");
  const [voice, setVoice] = useState("");
  const [clips, setClips] = useState<{ file: File; sourceName: string; text: string; language: string }[]>([]);
  const [preparing, setPreparing] = useState(false);
  const [importError, setImportError] = useState("");
  useEffect(() => {
    if (importError) feedback.reportIssue(`training-import-${importError}`, "训练素材导入失败", importError);
    return () => { if (importError) feedback.clearIssue(`training-import-${importError}`); };
  }, [importError, feedback]);
  const importGeneration = useRef(0);
  const [textMode, setTextMode] = useState(initialPreferences.textMode);
  const [transcribing, setTranscribing] = useState(false);
  const [transcriptionErrors, setTranscriptionErrors] = useState<Record<number, string>>({});
  const transcription = useRef<AbortController | null>(null);
  const transcriptionGeneration = useRef(0);
  const currentClips = useRef(clips);
  currentClips.current = clips;
  const [reviewed, setReviewed] = useState(false);
  useEffect(() => {
    setPreparing(false); setTranscribing(false); setTranscriptionErrors({}); setReviewed(false);
    return () => { importGeneration.current++; transcriptionGeneration.current++; transcription.current?.abort(); transcription.current = null; };
  }, [client]);
  const [epochs, setEpochs] = useState(initialPreferences.epochs);
  const [text, setText] = useState(initialPreferences.auditionText);
  useEffect(() => {
    saveTrainingPreferences(resources.baseUrl, { epochs, performancePreset, performance, textMode, auditionText: text });
  }, [resources.baseUrl, epochs, performancePreset, performance, textMode, text]);
  const voiceAvailable = c.voices.some(item => item.id === voice && item.available);
  useEffect(() => { if (!voiceAvailable) { setVoice(""); setReviewed(false); } }, [voiceAvailable]);
  const disabled = c.pending || c.snapshot?.busy || !c.snapshot;
  const validText = clips.every(x => x.text.trim() && Array.from(x.text).length <= 500 && !/[|\u0000-\u001f\u007f-\u009f]/u.test(x.text));
  const hasBlankText = textMode && clips.some(x => !x.text.trim());
  const canTranscribe = !disabled && !preparing && !transcribing && !importError && c.snapshot?.enabled && clips.length >= 2;
  const ready = canTranscribe && name.trim() && voiceAvailable && (!textMode || (reviewed && validText));
  function cancelTranscription() {
    transcriptionGeneration.current++;
    transcription.current?.abort(); transcription.current = null;
    setTranscribing(false); setTranscriptionErrors({}); setReviewed(false);
  }
  async function transcribeBlankClips() {
    if (!canTranscribe || !hasBlankText || transcription.current) return;
    const trigger = document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
    const controller = new AbortController();
    const generation = ++transcriptionGeneration.current;
    transcription.current = controller;
    const isCurrent = () => transcriptionGeneration.current === generation && !controller.signal.aborted;
    const failures: string[] = [];
    let completed = 0;
    setTranscribing(true); setTranscriptionErrors({}); setReviewed(false);
    try {
      for (const [index, clip] of clips.entries()) {
        if (!isCurrent()) return;
        if (currentClips.current[index]?.text.trim()) continue;
        try {
          const result = await client.transcribe(clip.file, clip.language, controller.signal);
          if (!isCurrent()) return;
          completed++;
          setClips(previous => isCurrent() ? previous.map((item, i) => i === index && item.file === clip.file && item.language === clip.language && !item.text.trim() ? { ...item, text: result.text } : item) : previous);
        } catch (error) {
          if (!isCurrent()) return;
          const message = error instanceof Error ? error.message : "文本提取失败";
          failures.push(`片段 ${index + 1}（${clip.sourceName}）：${message}`);
          setTranscriptionErrors(previous => ({ ...previous, [index]: message }));
        }
      }
      if (!isCurrent()) return;
      if (failures.length) {
        c.reportFailure("文本提取", new Error(`文本提取结果：${completed} 个片段成功，${failures.length} 个片段失败。\n${failures.join("\n")}`), trigger);
      } else {
        c.notify({ kind: "success", title: "文本提取成功", message: `已完成 ${completed} 个片段的文本提取，手工填写的内容已保留。`, tips: ["请逐片听取并校对文本，确认素材与所选音色一致后再开始训练。"] }, trigger);
      }
    } finally {
      if (isCurrent()) { transcription.current = null; setTranscribing(false); }
    }
  }
  function deleteVersion(id: string, title: string) {
    if (disabled || !window.confirm(`确定删除“${title}”吗？此版本的模型、训练素材和保存记录将永久删除，参考音色仍会保留。`)) return;
    void c.deleteVersion(id);
  }
  async function prepareClips(files: File[]) {
    cancelTranscription();
    const generation = ++importGeneration.current;
    setReviewed(false); setClipPage(0); setClipsExpanded(true); setClips([]); setImportError(""); setPreparing(false);
    if (files.length === 0) return;
    if (files.length < 2 || files.length > 32) { setImportError("请选择 2–32 个训练片段"); return; }
    const oversized = files.find(file => file.size > MAX_AUDIO_SOURCE_BYTES);
    if (oversized) { setImportError(`${oversized.name}：原文件不能超过 20 MiB`); return; }
    if (files.reduce((total, file) => total + file.size, 0) > MAX_TRAINING_BYTES) {
      setImportError("训练原文件总量不能超过 32 MiB"); return;
    }
    setPreparing(true);
    const prepared: typeof clips = [];
    let total = 0;
    let sourceName = "";
    try {
      for (const source of files) {
        sourceName = source.name;
        const { file } = await prepareAudioFile(source);
        if (generation !== importGeneration.current) return;
        total += file.size;
        if (total > MAX_TRAINING_BYTES) throw new Error("转换后的音频总量不能超过 32 MiB，请减少片段");
        prepared.push({ file, sourceName, text: "", language: "zh" });
      }
      setClips(prepared);
      feedback.success("训练素材导入成功", `已准备 ${prepared.length} 个片段，请检查语言、训练音色与本次训练参数。`);
    } catch (error) {
      if (generation === importGeneration.current) setImportError(`${sourceName}：${error instanceof Error ? error.message : "音频导入失败"}`);
    } finally {
      if (generation === importGeneration.current) setPreparing(false);
    }
  }
  return <section className="live-workspace" aria-labelledby="training-heading">
    <div className="connection-card"><div><h2 id="training-heading">训练状态</h2></div><span className="connection-pill">{c.snapshot?.busy ? "训练运行中" : c.snapshot?.enabled ? "训练就绪" : "训练未配置"}</span></div>
    {c.queryError && <p className="error-banner" role={feedback.enabled ? undefined : "alert"}>{c.queryError}</p>}
    {c.notice && <TrainingResultDialog key={c.notice.id} notice={c.notice} onClose={c.dismissNotice} />}
    {c.failedDelete && <button type="button" disabled={disabled} onClick={() => { if (c.failedDelete) void c.deleteVersion(c.failedDelete); }}>重试删除</button>}
    {tab === "create" && importError && <p className="error-banner" role={feedback.enabled ? undefined : "alert"}>{importError}</p>}
    <section className="connection-card training-model-runtime" aria-labelledby="training-models-heading">
      <div><h3 id="training-models-heading">语音模型</h3>
        <p role="status">{runtime?.message ?? "正在读取模型状态…"}</p>
        <p className="field-hint">播报前启用，训练前关闭。</p>
        {runtime?.state === "unsupported" && <p className="field-hint">当前引擎不支持独立启停；本机 TTS 更新后请重启服务。</p>}
      </div>
      <div className="form-actions"><button type="button" className={modelsLoaded ? undefined : "primary-button"} disabled={modelSwitchDisabled}
        onClick={() => { void c.setModelsEnabled(!modelsLoaded); }}>{modelsTransitioning ? runtime.state === "loading" ? "模型加载中…" : "模型关闭中…" : modelsLoaded ? "关闭语音模型" : "启用语音模型"}</button>
        {runtime?.state === "failed" && <button type="button" disabled={modelSwitchDisabled} onClick={() => { void c.setModelsEnabled(false); }}>清理模型内存</button>}
      </div>
    </section>
    <div role="tablist" aria-label="训练工作区" className="model-tabs training-tabs">
      {tabs.map(([id, title], index) => <button key={id} id={`training-tab-${id}`} type="button" role="tab" aria-selected={tab === id} aria-controls={`training-pane-${id}`} tabIndex={tab === id ? 0 : -1}
        onClick={() => setTab(id)} onKeyDown={event => {
          const next = event.key === "ArrowRight" ? (index + 1) % tabs.length : event.key === "ArrowLeft" ? (index + tabs.length - 1) % tabs.length : event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : -1;
          if (next < 0) return;
          event.preventDefault(); setTab(tabs[next][0]); document.getElementById(`training-tab-${tabs[next][0]}`)?.focus();
        }}>{title}{id === "history" ? ` (${jobs.length})` : id === "voices" ? ` (${voiceGroups.length})` : ""}</button>)}
    </div>
    <div role="tabpanel" id={`training-pane-${tab}`} aria-labelledby={`training-tab-${tab}`}>
    {tab === "create" && <section className="panel" aria-labelledby="training-config-heading">
      <h3 id="training-config-heading">新建训练配置</h3>
      {c.activeVoiceId !== undefined && !c.voices.some(item => item.available) && <p className="availability-note">
        请先<a href="#resources">上传参考声音</a>。
      </p>}
      <p className="availability-note">训练前结束直播，关闭语音模型与本地 LLM，释放显存。</p>
      <form onSubmit={e => {
        e.preventDefault();
        if (hasBlankText) { void transcribeBlankClips(); return; }
        if (!ready) return;
        void c.action(async signal => {
          const job = await client.create({ name, voice_id: voice, sovits_epochs: epochs, gpt_epochs: epochs, text_mode: textMode ? "reviewed_text" : "audio_only", reviewed: textMode && reviewed, performance,
            clips: clips.map(({ text, language }) => ({ text: textMode ? text : "", language })) }, clips.map(x => x.file), signal);
          if (!signal.aborted) c.trackJob(job);
        }, { operation: "提交训练", successTitle: "训练已提交", success: "任务已接收，请到“训练记录”查看进度。训练完成或失败时会另外通知。" });
      }}>
        <div className="training-fields">
          <label>训练音色<select value={voice} onChange={e => { setVoice(e.target.value); setReviewed(false); }} disabled={c.pending} aria-describedby="training-voice-hint"><option value="">请选择本次训练的音色</option>{c.voices.filter(v => v.available).map(v => <option key={v.id} value={v.id}>{v.name}</option>)}</select></label>
          <label>训练名称<input value={name} maxLength={80} onChange={e => setName(e.target.value)} disabled={c.pending} /></label>
          <label>GPT / SoVITS 轮次<input type="number" min={1} max={20} value={epochs} onChange={e => setEpochs(bounded(e.target.value, 1, 20))} disabled={c.pending} /></label>
          <label>训练片段<input type="file" accept={AUDIO_FILE_ACCEPT} multiple disabled={c.pending} onChange={e => {
            const files = Array.from(e.target.files ?? []); e.target.value = "";
            void prepareClips(files);
          }} /></label>
        </div>
        <p className="field-hint">2–32 段录音 · 每段 3–10 秒 · 单文件 ≤20 MiB · 原文件及转换后总量均 ≤32 MiB</p>
        <fieldset className="training-performance" disabled={c.pending}><legend>硬件与性能</legend>
          <label>性能预设<select value={performancePreset} onChange={event => {
            const key = event.target.value as TrainingPerformancePreset; setPerformancePreset(key);
            if (key in performancePresets) setPerformance(previous => ({ ...previous, ...performancePresets[key as keyof typeof performancePresets] }));
          }}><option value="low">省内存</option><option value="balanced">均衡</option><option value="high">高吞吐</option><option value="custom">自定义</option></select></label>
          <p className="field-hint">显存较小时选择“省内存”。</p>
          <details className="training-details" open={performancePreset === "custom"}><summary>高级训练参数</summary><div className="training-fields">
            {([["batch_size", "每批片段数", 1, 16], ["data_workers", "数据加载进程", 0, 8], ["cpu_threads", "CPU 线程数", 1, 16]] as const).map(([key, title, min, max]) => <label key={key}>{title}<input type="number" min={min} max={max} value={performance[key]} onChange={event => { setPerformancePreset("custom"); setPerformance(previous => ({ ...previous, [key]: bounded(event.target.value, min, max) })); }} /></label>)}
          </div></details>
          <div className="training-fields"><label>GPU 编号<input type="number" min={0} max={15} value={performance.gpu_index} onChange={event => setPerformance(previous => ({ ...previous, gpu_index: bounded(event.target.value, 0, 15) }))} /></label>
            <label className="training-checkbox"><input type="checkbox" checked={performance.low_memory} onChange={event => setPerformance(previous => ({ ...previous, low_memory: event.target.checked }))} />节省显存</label></div>
          <p className="field-hint">每批 {performance.batch_size} 段 · {performance.data_workers} 个加载进程 · {performance.cpu_threads} 个 CPU 线程</p>
        </fieldset>
        <p id="training-voice-hint" className="field-hint">已有成功版本时会继续训练，新版本单独保存。</p>
        <label><input type="checkbox" checked={textMode} disabled={c.pending} onChange={e => { cancelTranscription(); setTextMode(e.target.checked); }} />输入并校对文本</label>
        <p className="field-hint">{textMode ? "留空可先提取文本，训练前逐片校对。" : "仅声音训练需配置本地语音识别。"}</p>
        {c.snapshot?.enabled === false && <p className="availability-note">训练未配置，可先准备素材。</p>}
        {preparing && <p role="status">正在准备音频，请稍候…可以重新选择片段。</p>}
        {transcribing && <p role="status">正在提取空白片段文本，请稍候…你仍可手工填写文本。</p>}
        {clips.length > 0 && <p><button type="button" aria-expanded={clipsExpanded} aria-controls="training-clip-editor" onClick={() => setClipsExpanded(previous => !previous)}>{clipsExpanded ? "收起片段编辑" : "展开片段编辑"}</button><span className="field-hint"> 已准备 {clips.length} 个片段</span></p>}
        {clipsExpanded && <div id="training-clip-editor">{clips.slice(clipPage * PAGE_SIZE, (clipPage + 1) * PAGE_SIZE).map((clip, offset) => { const index = clipPage * PAGE_SIZE + offset; return <fieldset key={`${index}-${clip.sourceName}`} disabled={c.pending || preparing} className="training-clip"><legend>{clip.sourceName}</legend>
          {textMode && <label>片段 {index + 1} 文本<input value={clip.text} maxLength={500} onChange={e => {
            const value = e.target.value;
            setReviewed(false); setClips(previous => previous.map((item, i) => i === index ? { ...item, text: value } : item));
            setTranscriptionErrors(previous => { const next = { ...previous }; delete next[index]; return next; });
          }} /></label>}
          <label>片段 {index + 1} 语言<select value={clip.language} onChange={e => { const language = e.target.value; cancelTranscription(); setClips(previous => previous.map((item, i) => i === index ? { ...item, language } : item)); }}>{[["zh", "中文"], ["en", "英语"], ["ja", "日语"], ["ko", "韩语"], ["yue", "粤语"]].map(([value, title]) => <option key={value} value={value}>{title}</option>)}</select></label>
          {textMode && transcriptionErrors[index] && !clip.text.trim() && <p className="field-error">{clip.sourceName}：{transcriptionErrors[index]}。可重新提取空白片段文本，或手工填写。</p>}
        </fieldset>; })}</div>}
        {clips.length > 0 && <Pagination label="训练片段" count={clips.length} page={clipPage} onPage={setClipPage} />}
        {textMode && <label><input type="checkbox" checked={reviewed && voiceAvailable} disabled={c.pending || preparing || transcribing || clips.length < 2 || !voiceAvailable || !validText} onChange={e => setReviewed(e.target.checked)} />我已逐片听取并核对文本，素材与所选音色一致</label>}
        <p><button type="submit" className="primary-button" disabled={hasBlankText ? !canTranscribe : !ready}>{transcribing ? "正在提取文本…" : hasBlankText ? "提取空白片段文本" : "开始训练"}</button></p>
      </form>
    </section>}
    {tab === "history" && <section className="panel" aria-labelledby="training-history-heading"><h3 id="training-history-heading">训练记录</h3>
    <p className="muted">完成后前往“已训练音色”试听。</p>
    {jobs.length === 0 && <p className="field-hint">还没有训练记录。</p>}
    <ul className="training-list" aria-label="训练任务">{jobs.slice(currentJobPage * PAGE_SIZE, (currentJobPage + 1) * PAGE_SIZE).map(job => <li key={job.id}>
      <strong>{job.name}</strong> · {phases[job.status] ?? job.status} <progress max={100} value={job.progress} aria-label={`${job.name} 进度`} />
      <small>{job.clip_count} 个片段 · {new Date(job.created_at_ms).toLocaleString()}</small>
      <details className="training-details"><summary>查看任务详情</summary><p>{job.message}</p>
        <p className="field-hint">批量 {job.performance.batch_size} · 加载进程 {job.performance.data_workers} · CPU 线程 {job.performance.cpu_threads} · GPU {job.performance.gpu_index} · {job.performance.low_memory ? "节省显存" : "常规显存模式"}</p>
        {terminalFeedback(job) && <button type="button" onClick={() => { const feedback = terminalFeedback(job); if (feedback) c.notify(feedback); }}>查看结果与建议</button>}
      </details>
      {!["completed", "failed", "cancelled", "interrupted"].includes(job.status) && <button type="button" disabled={c.pending || job.status === "cancelling"} onClick={() => { void c.action(async signal => { await client.cancel(job.id, signal); }, { operation: "取消训练", successTitle: "取消请求已提交", success: "请等待训练进程停止，最终结果会显示在训练记录中。" }); }}>取消训练</button>}
      {["failed", "cancelled", "interrupted"].includes(job.status) && <button type="button" disabled={disabled} aria-label={`删除任务 ${job.name}`} onClick={() => deleteVersion(job.id, job.name)}>删除任务</button>}
    </li>)}</ul>
    <Pagination label="训练记录" count={jobs.length} page={currentJobPage} onPage={setJobPage} />
    </section>}
    {tab === "voices" && <TrainingVoiceLibrary controller={c} disabled={!!disabled} canSynthesize={canSynthesize} text={text} onText={setText} deleteVersion={deleteVersion} />}
    {tab === "offline" && <section className="connection-card" aria-labelledby="training-offline-heading"><div><h3 id="training-offline-heading">离线预设</h3><p>{c.preset?.message ?? "正在读取运行预设…"}</p><p>{c.preset?.model || "未配置模型"} · {c.preset?.verified ? "本次实测通过" : "未验证"}</p>
      {c.preset?.measurement && <dl className="live-metrics"><div><dt>LLM</dt><dd>{c.preset.measurement.llm_ms} ms</dd></div><div><dt>TTS</dt><dd>{c.preset.measurement.tts_ms} ms</dd></div><div><dt>总耗时</dt><dd>{c.preset.measurement.total_ms} ms</dd></div><div><dt>显存峰值</dt><dd>{c.preset.measurement.gpu_peak_used_mib} / {c.preset.measurement.gpu_total_mib} MiB</dd></div></dl>}
      <label>测量参考音色<select value={voice} onChange={event => setVoice(event.target.value)} disabled={c.pending}><option value="">请选择音色</option>{c.voices.filter(item => item.available).map(item => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label>
      <label>试听与测量文本<textarea value={text} maxLength={500} onChange={event => setText(event.target.value)} /></label>
      {!canSynthesize && <p className="availability-note">测量前请先启动 TTS 服务并启用上方语音模型。</p>}
      <button type="button" disabled={disabled || !canSynthesize || c.preset?.mode !== "local" || !voiceAvailable || !text.trim()} onClick={() => { void c.measure(voice, text); }}>测量本地 LLM 与 TTS</button>
    </div></section>}
    </div>
  </section>;
}
