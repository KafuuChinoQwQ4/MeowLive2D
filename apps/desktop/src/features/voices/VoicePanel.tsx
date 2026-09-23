import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import type { VoiceCreateRequest, VoiceProfile } from "@meowlive/contracts";
import type { VoiceController } from "./types";
import { useFeedback } from "../../app/feedback/OperationFeedback";
import { AUDIO_FILE_ACCEPT, prepareAudioFile } from "../../services/audio";
import { createTrainingClient, type TrainingClient } from "../../services/server/training";
import { useReferenceTranscription } from "./useReferenceTranscription";

const defaultTranscriber = createTrainingClient();

const languages = [
  ["zh", "中文"], ["en", "英语"], ["ja", "日语"], ["ko", "韩语"], ["yue", "粤语"], ["auto", "自动识别"],
] as const;

const previewLabels = {
  queued: "试听已加入播报队列",
  synthesizing: "试听正在合成",
  ready: "试听等待播放",
  playing: "试听正在播放",
  completed: "试听已完成",
  cancelled: "试听已取消",
  failed: "试听失败",
  unknown: "试听结果未知",
} as const;

function VoiceRow({ voice, active, busy, controller }: {
  voice: VoiceProfile;
  active: boolean;
  busy: boolean;
  controller: VoiceController;
}) {
  return <li className="resource-row">
    <div className="resource-row-heading">
      <div><strong>{voice.name}</strong><span className="field-hint">{voice.language} · {(voice.duration_ms / 1_000).toFixed(1)} 秒</span></div>
      {active && <span className="task-status task-completed">当前音色</span>}
    </div>
    <p className="resource-reference">{voice.reference_text}</p>
    {!voice.available && <p className="field-error">{voice.error ?? "音色文件缺失，请重新上传。"}</p>}
    <div className="compact-actions">
      <button type="button" className="primary-button" disabled={busy || active || !voice.available}
        onClick={() => { void controller.selectVoice(voice.id); }}>设为当前音色</button>
      <button type="button" disabled={busy || !voice.available}
        onClick={() => { void controller.previewVoice(voice.id, voice.reference_text); }}>试听</button>
      <button type="button" className="stop-button" disabled={busy} aria-label={`删除音色 ${voice.name}`}
        onClick={() => {
          if (window.confirm(`删除音色“${voice.name}”及参考音频？关联角色的音色将清空。此操作无法撤销。`)) void controller.deleteVoice(voice.id);
        }}>删除</button>
    </div>
  </li>;
}

export function VoicePanel({ controller, transcriber = defaultTranscriber }: { controller: VoiceController; transcriber?: Pick<TrainingClient, "transcribe"> }) {
  const feedback = useFeedback();
  const [metadata, setMetadata] = useState<Pick<VoiceCreateRequest, "name" | "language">>({ name: "", language: "zh" });
  const audioGeneration = useRef(0);
  useEffect(() => () => { audioGeneration.current += 1; }, []);
  const input = useRef<HTMLInputElement>(null);
  const [audio, setAudio] = useState<File | null>(null);
  const transcription = useReferenceTranscription(audio, metadata.language, transcriber);
  const [audioSummary, setAudioSummary] = useState<string | null>(null);
  const [audioError, setAudioError] = useState<string | null>(null);
  const [preparingAudio, setPreparingAudio] = useState(false);
  const snapshot = controller.snapshot;
  const busy = controller.pendingAction !== null;
  const validMetadata = metadata.name.trim().length > 0
    && metadata.name.trim().length <= 80
    && transcription.text.trim().length > 0
    && Array.from(transcription.text).length <= 500;

  async function chooseAudio(file: File | undefined) {
    const generation = ++audioGeneration.current;
    setAudio(null);
    setAudioSummary(null);
    setAudioError(null);
    setPreparingAudio(Boolean(file));
    if (!file) return;
    try {
      const { file: prepared, info } = await prepareAudioFile(file);
      if (generation !== audioGeneration.current) return;
      setAudio(prepared);
      setAudioSummary(`${(info.durationMs / 1_000).toFixed(1)} 秒 · ${info.sampleRate / 1_000} kHz · ${info.channels === 1 ? "单声道" : "双声道"}`);
      feedback.success("参考音频已准备", `${file.name} 已通过音频检查，空白参考文本会自动提取，请核对原文后上传。`);
    } catch (error) {
      if (generation !== audioGeneration.current) return;
      setAudioError(error instanceof Error ? error.message : "无法读取音频文件");
      feedback.error("参考音频处理失败", error, "无法读取音频文件");
    } finally {
      if (generation === audioGeneration.current) setPreparingAudio(false);
    }
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (busy) return;
    if (!audio || !validMetadata) { feedback.error("音色检查失败", !audio ? "请先选择并通过检查的参考音频。" : "请填写 1–80 字的音色名称和 1–500 字的参考文本。"); return; }
    const value = await controller.uploadVoice({
      name: metadata.name.trim(), language: metadata.language, reference_text: transcription.text.trim(),
    }, audio);
    if (value) {
      setMetadata({ name: "", language: "zh" });
      transcription.setText("");
      audioGeneration.current += 1;
      if (input.current) input.current.value = "";
      setAudio(null);
      setAudioSummary(null);
    }
  }

  return <section className="panel" aria-labelledby="voices-heading">
    <h2 id="voices-heading">音色管理</h2>
    <p className="field-hint">参考音色保存后即可使用；训练版本在下方「已训练音色」页签切换。</p>
    {snapshot && !snapshot.default_voice_available && snapshot.voices.length === 0 && <p className="availability-note" role="status">
      尚未设置音色，请先上传参考录音，再按需在下方训练。
    </p>}

    <ul className="resource-list" aria-label="可用音色">
      {snapshot?.default_voice_available && <li className="resource-row">
        <div className="resource-row-heading"><strong>配置的默认音色</strong>{snapshot.active_voice_id === "default" && <span className="task-status task-completed">当前音色</span>}</div>
        <div className="compact-actions"><button type="button" className="primary-button"
          disabled={busy || snapshot.active_voice_id === "default"}
          onClick={() => { void controller.selectVoice("default"); }}>设为当前音色</button></div>
      </li>}
      {snapshot?.voices.map((voice) => <VoiceRow key={voice.id} voice={voice}
        active={snapshot.active_voice_id === voice.id} busy={busy} controller={controller} />)}
    </ul>

    {controller.voiceDeleteRetries.map(id => <div key={id} className="availability-note" role="status">
      音色配置已删除，参考文件尚未清理完成。
      <button type="button" disabled={busy} onClick={() => { void controller.deleteVoice(id); }}>重试清理参考音频</button>
    </div>)}

    {controller.voicePreview && <p className={["failed", "unknown"].includes(controller.voicePreview.status) ? "field-error" : "success-banner"} role="status">
      {previewLabels[controller.voicePreview.status]}
      {controller.voicePreview.error ? `：${controller.voicePreview.error}` : ""} <a href="#history-heading">查看播报状态</a>
    </p>}

    <div className="panel-divider" />
    <h3>上传新音色</h3>
    <form onSubmit={(event) => { void submit(event); }}>
      <label htmlFor="voice-name">音色名称</label>
      <input id="voice-name" value={metadata.name} maxLength={80} disabled={busy}
        onChange={(event) => setMetadata((current) => ({ ...current, name: event.target.value }))} />
      <label htmlFor="voice-language">参考文本语言</label>
      <select id="voice-language" value={metadata.language} disabled={busy}
        onChange={(event) => setMetadata((current) => ({ ...current, language: event.target.value }))}>
        {languages.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
      </select>
      <label htmlFor="voice-reference">参考文本</label>
      <textarea id="voice-reference" rows={3} maxLength={500} value={transcription.text} disabled={busy}
        onChange={(event) => transcription.setText(event.target.value)} />
      <p className="field-hint">选择音频后自动提取空白参考文本。请核对录音原文，语言需一致；也可手工填写。</p>
      {!transcription.supported && <p className="field-hint">自动转写前请选择录音的具体语言；也可直接手工填写参考文本。</p>}
      {transcription.transcribing && <p role="status">正在本地提取参考文本…</p>}
      {transcription.error && <p className="field-error" role="alert">{transcription.error}。可重试或手工填写。</p>}
      <button type="button" disabled={!audio || busy || transcription.transcribing || !transcription.supported || !!transcription.text.trim()}
        onClick={transcription.retry}>提取参考文本</button>
      <label htmlFor="voice-audio">参考音频</label>
      <input ref={input} id="voice-audio" type="file" accept={AUDIO_FILE_ACCEPT} disabled={busy}
        onChange={(event) => { void chooseAudio(event.target.files?.[0]); }} />
      <p className="field-hint">3–10 秒非静音录音 · 原文件 ≤20 MiB · 转换后 ≤2 MiB。</p>
      {preparingAudio && <p className="field-hint" role="status">正在处理音频…</p>}
      {audioSummary && <p className="success-banner" role="status">音频有效：{audioSummary}</p>}
      {audioError && <p className="field-error" role="alert">{audioError}</p>}
      <div className="form-actions"><button className="primary-button" type="submit" disabled={!audio || !validMetadata || busy}>
        {controller.pendingAction === "voice-upload" ? "正在上传…" : "上传音色"}
      </button></div>
    </form>
  </section>;
}
