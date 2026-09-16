import { useEffect, useState } from "react";
import { createTrainingClient, type TrainingClient } from "../../services/server/training";
import { createResourceClient, type ResourcesClient } from "../../services/server/resources";
import { useTraining } from "./useTraining";
const defaultClient = createTrainingClient();
const defaultResources = createResourceClient();
const phases: Record<string, string> = { queued: "排队", preparing: "准备素材", training: "训练中", validating: "校验权重", completed: "训练完成", failed: "训练失败", cancelling: "取消中", cancelled: "已取消", interrupted: "已中断" };

export function TrainingPanel({ client = defaultClient, resources = defaultResources }: { client?: TrainingClient; resources?: ResourcesClient }) {
  const c = useTraining(client, resources);
  const [name, setName] = useState("");
  const [voice, setVoice] = useState("");
  const [clips, setClips] = useState<{ file: File; text: string; language: string }[]>([]);
  const [reviewed, setReviewed] = useState(false);
  const [epochs, setEpochs] = useState(1);
  const [text, setText] = useState("你好，欢迎来到直播间，希望你今天过得愉快。");
  const [listened, setListened] = useState<string[]>([]);
  const [savedVoice, setSavedVoice] = useState("");
  const savedVersions = c.snapshot?.versions.filter(version => version.saved) ?? [];
  const selectedVersion = savedVersions.find(version => version.id === savedVoice);
  const currentVersion = c.snapshot?.versions.find(version => version.active && version.voice_id === c.activeVoiceId);
  const hasReference = (voiceId: string) => c.voices.some(item => item.id === voiceId && item.available);
  const voiceAvailable = c.voices.some(item => item.id === voice && item.available);
  useEffect(() => { if (!voiceAvailable) setReviewed(false); }, [voiceAvailable]);
  const disabled = c.pending || c.snapshot?.busy || !c.snapshot;
  const ready = !disabled && c.snapshot?.enabled && name.trim() && voiceAvailable && clips.length >= 2 && reviewed && clips.every(x => x.text.trim() && !/[|\r\n]/u.test(x.text));
  return <section className="live-workspace" aria-labelledby="training-heading">
    <div className="connection-card"><div><p className="eyebrow">音色进阶</p><h2 id="training-heading">音色微调与离线预设</h2></div><span className="connection-pill">{c.snapshot?.busy ? "训练运行中" : c.snapshot?.enabled ? "训练就绪" : "训练未配置"}</span></div>
    {c.error && <p className="error-banner" role="alert">{c.error}</p>}
    <section className="panel" aria-labelledby="saved-voices-heading">
      <h3 id="saved-voices-heading">已保存音色</h3>
      <p className="muted">训练完成后，试听确认并保存音色。下次打开可直接选择使用，也可随时切换其他已保存音色，无需重新训练。</p>
      <p role="status">当前使用：{currentVersion?.name ?? c.voices.find(item => item.id === c.activeVoiceId)?.name ?? (c.activeVoiceId === undefined ? "正在读取…" : c.activeVoiceId === "default" && c.defaultVoiceAvailable ? "配置的默认音色" : "未设置音色")}</p>
      {savedVersions.length === 0 && <p className="field-hint">还没有保存的训练音色。请先上传自己的素材并完成训练，再试听确认、保存并选用音色。</p>}
      <label htmlFor="saved-voice">选择已保存音色</label>
      <select id="saved-voice" value={savedVoice} disabled={disabled || savedVersions.length === 0} onChange={e => setSavedVoice(e.target.value)}>
        <option value="">选择一个已保存音色</option>
        {savedVersions.map(version => <option key={version.id} value={version.id} disabled={!version.available || !hasReference(version.voice_id)}>
          {version.name} · {c.voices.find(item => item.id === version.voice_id)?.name ?? "参考音色缺失"}{!version.available ? "（模型不可用）" : !hasReference(version.voice_id) ? "（参考音频不可用）" : ""}
        </option>)}
      </select>
      <div className="form-actions"><button type="button" className="primary-button"
        disabled={disabled || !selectedVersion?.available || !hasReference(selectedVersion.voice_id) || currentVersion?.id === selectedVersion.id}
        onClick={() => { if (selectedVersion) void c.activateVersion(selectedVersion); }}>使用所选音色</button></div>
      <p className="field-hint">选用后，文字播报与 Agent 使用该音色；已有播报结束后再切换。</p>
    </section>
    {c.activeVoiceId !== undefined && !c.voices.some(item => item.available) && <p className="availability-note">
      开始训练前，请先前往音色管理<a href="#resources">上传参考声音</a>并填写对应文本，再回到这里选择音色、上传至少两个训练片段并逐片核对文本。
    </p>}
    <p className="availability-note">训练前请结束直播并停止 TTS、本地 LLM 推理，释放显存。每个片段需 3–10 秒 PCM16 WAV，2–32 个片段、总量不超过 32 MiB。训练结果需要试听确认后启用。</p>
    <form onSubmit={e => { e.preventDefault(); if (!ready) return; void c.action(async signal => { await client.create({ name, voice_id: voice, sovits_epochs: epochs, gpt_epochs: epochs, reviewed, clips: clips.map(({ text, language }) => ({ text, language })) }, clips.map(x => x.file), signal); }); }}>
      <div className="training-fields">
        <label>训练名称<input value={name} maxLength={80} onChange={e => setName(e.target.value)} disabled={disabled} /></label>
        <label>训练音色<select value={voice} onChange={e => { setVoice(e.target.value); setReviewed(false); }} disabled={disabled}><option value="">选择已有音色</option>{c.voices.filter(v => v.available).map(v => <option key={v.id} value={v.id}>{v.name}</option>)}</select></label>
        <label>GPT / SoVITS 轮次<input type="number" min={1} max={20} value={epochs} onChange={e => setEpochs(Math.min(20, Math.max(1, Number(e.target.value) || 1)))} disabled={disabled} /></label>
        <label>训练片段<input type="file" accept=".wav,audio/wav" multiple disabled={disabled} onChange={e => {
          const files = Array.from(e.target.files ?? []); setReviewed(false);
          if (files.length > 32 || files.some(f => f.size > 2097152) || files.reduce((n, f) => n + f.size, 0) > 33554432) { c.setError("片段数量或大小超限"); setClips([]); return; }
          setClips(files.map(file => ({ file, text: "", language: "zh" })));
        }} /></label>
      </div>
      {clips.map((clip, index) => <fieldset key={`${index}-${clip.file.name}`} disabled={disabled} className="training-clip"><legend>{clip.file.name}</legend>
        <label>片段 {index + 1} 文本<input value={clip.text} maxLength={500} onChange={e => { setReviewed(false); setClips(clips.map((item, i) => i === index ? { ...item, text: e.target.value } : item)); }} /></label>
        <label>片段 {index + 1} 语言<select value={clip.language} onChange={e => { setReviewed(false); setClips(clips.map((item, i) => i === index ? { ...item, language: e.target.value } : item)); }}>{[["zh", "中文"], ["en", "英语"], ["ja", "日语"], ["ko", "韩语"], ["yue", "粤语"]].map(([value, title]) => <option key={value} value={value}>{title}</option>)}</select></label>
      </fieldset>)}
      <label><input type="checkbox" checked={reviewed && voiceAvailable} disabled={disabled || !voiceAvailable} onChange={e => setReviewed(e.target.checked)} />我已逐片听取并核对文本，素材与所选音色一致</label>
      <p><button type="submit" className="primary-button" disabled={!ready}>开始训练</button></p>
    </form>
    <ul className="training-list" aria-label="训练任务">{c.snapshot?.jobs.slice().reverse().map(job => <li key={job.id}>
      <strong>{job.name}</strong> · {phases[job.status] ?? job.status} <progress max={100} value={job.progress} aria-label={`${job.name} 进度`} />
      <p>{job.message}</p><small>{job.clip_count} 个片段 · {new Date(job.created_at_ms).toLocaleString()}</small>
      {!["completed", "failed", "cancelled", "interrupted"].includes(job.status) && <button type="button" disabled={c.pending || job.status === "cancelling"} onClick={() => { void c.action(async signal => { await client.cancel(job.id, signal); }); }}>取消训练</button>}
    </li>)}</ul>
    <label>试听与测量文本<textarea value={text} maxLength={500} onChange={e => setText(e.target.value)} /></label>
    <p className="field-hint">完成的训练模型会自动保留在本机，保存音色不会覆盖其他版本。</p>
    <ul className="training-list" aria-label="模型版本">{c.snapshot?.versions.map(version => <li key={version.id}><strong>{version.name}</strong> · GPT-SoVITS v2 {version.active && <span>{c.activeVoiceId === version.voice_id ? "当前使用" : "该参考音色的已启用版本"}</span>}
      {!version.available && <p className="error-banner">模型文件缺失或损坏</p>}
      {!hasReference(version.voice_id) && <p className="field-error">参考音频不可用，请检查音色资源。</p>}
      <div className="resource-actions"><button type="button" disabled={disabled || !version.available || !hasReference(version.voice_id) || !text.trim()} onClick={() => { void c.audition(version.id, text); }}>生成版本试听</button>
      <button type="button" disabled={disabled || version.saved || !version.available || !version.auditioned || !listened.includes(version.id)} onClick={() => { void c.saveVersion(version.id); }}>{version.saved ? "音色已保存" : "保存音色"}</button>
      <button type="button" disabled={disabled || !version.available || !hasReference(version.voice_id) || !version.auditioned || (!version.saved && !listened.includes(version.id)) || currentVersion?.id === version.id} onClick={() => { void c.activateVersion(version); }}>启用此版本</button></div>
      {c.audio?.version === version.id && <><audio controls src={c.audio.url} aria-label="版本试听音频" /><label><input type="checkbox" checked={listened.includes(version.id)} onChange={e => setListened(e.target.checked ? [...listened, version.id] : listened.filter(x => x !== version.id))} />我已试听并确认此版本效果</label></>}
    </li>)}</ul>
    <div className="connection-card"><div><h3>离线预设</h3><p>{c.preset?.message ?? "正在读取运行预设…"}</p><p>{c.preset?.model || "未配置模型"} · {c.preset?.verified ? "本次实测通过" : "未验证"}</p>
      {c.preset?.measurement && <dl className="live-metrics"><div><dt>LLM</dt><dd>{c.preset.measurement.llm_ms} ms</dd></div><div><dt>TTS</dt><dd>{c.preset.measurement.tts_ms} ms</dd></div><div><dt>总耗时</dt><dd>{c.preset.measurement.total_ms} ms</dd></div><div><dt>显存峰值</dt><dd>{c.preset.measurement.gpu_peak_used_mib} / {c.preset.measurement.gpu_total_mib} MiB</dd></div></dl>}
      <button type="button" disabled={disabled || c.preset?.mode !== "local" || !voiceAvailable || !text.trim()} onClick={() => { void c.action(async signal => { await client.measure(voice, text, signal); }); }}>测量本地 LLM 与 TTS</button>
    </div></div>
  </section>;
}
