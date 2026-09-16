import { useRef, useState } from "react";
import type { FormEvent } from "react";
import type { VoiceCreateRequest, VoiceProfile } from "@meowlive/contracts";
import type { VoiceController } from "./types";
import { validateVoiceWav } from "./wav";

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
    </div>
  </li>;
}

export function VoicePanel({ controller }: { controller: VoiceController }) {
  const [metadata, setMetadata] = useState<VoiceCreateRequest>({ name: "", language: "zh", reference_text: "" });
  const audioGeneration = useRef(0);
  const input = useRef<HTMLInputElement>(null);
  const [audio, setAudio] = useState<File | null>(null);
  const [audioSummary, setAudioSummary] = useState<string | null>(null);
  const [audioError, setAudioError] = useState<string | null>(null);
  const snapshot = controller.snapshot;
  const busy = controller.pendingAction !== null;
  const validMetadata = metadata.name.trim().length > 0
    && metadata.name.trim().length <= 80
    && metadata.reference_text.trim().length > 0
    && Array.from(metadata.reference_text).length <= 500;

  async function chooseAudio(file: File | undefined) {
    const generation = ++audioGeneration.current;
    setAudio(null);
    setAudioSummary(null);
    setAudioError(null);
    if (!file) return;
    try {
      const info = await validateVoiceWav(file);
      if (generation !== audioGeneration.current) return;
      setAudio(file);
      setAudioSummary(`${(info.durationMs / 1_000).toFixed(1)} 秒 · ${info.sampleRate / 1_000} kHz · ${info.channels === 1 ? "单声道" : "双声道"}`);
    } catch (error) {
      if (generation !== audioGeneration.current) return;
      setAudioError(error instanceof Error ? error.message : "无法读取 WAV 文件");
    }
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!audio || !validMetadata || busy) return;
    const value = await controller.uploadVoice({
      name: metadata.name.trim(), language: metadata.language, reference_text: metadata.reference_text.trim(),
    }, audio);
    if (value) {
      setMetadata({ name: "", language: "zh", reference_text: "" });
      audioGeneration.current += 1;
      if (input.current) input.current.value = "";
      setAudio(null);
      setAudioSummary(null);
    }
  }

  return <section className="panel" aria-labelledby="voices-heading">
    <p className="eyebrow">声音资源</p>
    <h2 id="voices-heading">音色管理</h2>
    <p className="muted">上传参考声音，选择直播使用的音色。</p>
    <p className="field-hint">训练过的声音可前往<a href="#training">已保存音色</a>直接选择和切换，无需重新训练。</p>
    {snapshot && !snapshot.default_voice_available && snapshot.voices.length === 0 && <p className="availability-note" role="status">
      尚未设置音色。请先在下方上传自己的参考音频并填写对应文本，再前往<a href="#training">训练初始音色</a>，上传训练片段、试听并保存。
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

    {controller.voicePreview && <p className={controller.voicePreview.status === "failed" ? "field-error" : "success-banner"} role="status">
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
      <textarea id="voice-reference" rows={3} maxLength={500} value={metadata.reference_text} disabled={busy}
        onChange={(event) => setMetadata((current) => ({ ...current, reference_text: event.target.value }))} />
      <p className="field-hint">请选择与录音内容和语言一致的文本。</p>
      <label htmlFor="voice-audio">参考音频</label>
      <input ref={input} id="voice-audio" type="file" accept=".wav,audio/wav" disabled={busy}
        onChange={(event) => { void chooseAudio(event.target.files?.[0]); }} />
      <p className="field-hint">PCM16 WAV，3–10 秒，8–48 kHz，不能静音，最大 2 MiB。</p>
      {audioSummary && <p className="success-banner" role="status">音频有效：{audioSummary}</p>}
      {audioError && <p className="field-error" role="alert">{audioError}</p>}
      <div className="form-actions"><button className="primary-button" type="submit" disabled={!audio || !validMetadata || busy}>
        {controller.pendingAction === "voice-upload" ? "正在上传…" : "上传音色"}
      </button></div>
    </form>
  </section>;
}
