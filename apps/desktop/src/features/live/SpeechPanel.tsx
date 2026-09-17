import { useState } from "react";
import type { FormEvent } from "react";
import type { SpeechStatus } from "@meowlive/contracts";
import { createServerClient } from "../../services/server";
import type { ServerClient } from "../../services/server";
import { isActiveSpeech, useSpeechController } from "./useSpeechController";
import { useFeedback } from "../../app/feedback/OperationFeedback";

const defaultClient = createServerClient();
const statusLabels: Record<SpeechStatus, string> = {
  queued: "排队中", synthesizing: "合成中", ready: "等待播放", playing: "播放中",
  completed: "已完成", cancelled: "已取消", failed: "失败", unknown: "结果未知",
};

export function SpeechPanel({ client = defaultClient, pollIntervalMs = 1_000 }: { client?: ServerClient; pollIntervalMs?: number }) {
  const feedback = useFeedback();
  const [text, setText] = useState("");
  const [voiceId, setVoiceId] = useState("active");
  const controller = useSpeechController(client, pollIntervalMs);
  const { status, connectionError, actionError, pendingAction } = controller;
  const textLength = Array.from(text.trim()).length;
  const validVoice = /^[a-zA-Z0-9_-]{1,64}$/.test(voiceId.trim());
  const available = Boolean(status?.bridge_connected) && !connectionError;
  const canSubmit = available && !pendingAction && textLength > 0 && textLength <= 500 && validVoice;
  const canStop = Boolean(status?.speeches.some((task) => isActiveSpeech(task.status))) && !pendingAction;
  const speeches = status?.speeches.slice().reverse() ?? [];

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit) {
      if (!pendingAction) feedback.error("播报检查失败", !available ? "请先连接桌面执行端。" : !validVoice ? "声音 ID 须为 1–64 位字母、数字、下划线或连字符。" : "播报文本须为 1–500 字。");
      return;
    }
    const accepted = await controller.submit({ text: text.trim(), voice_id: voiceId.trim() });
    if (accepted) setText("");
  }

  return (
    <div className="speech-workspace">
      <section className="connection-card" aria-labelledby="connection-heading">
        <div>
          <h2 id="connection-heading">连接状态</h2>
          <p className="server-address">{client.baseUrl}</p>
        </div>
        <div className="connection-indicators" role="status" aria-live="polite">
          <span className={`connection-pill ${status && !connectionError ? "connected" : "disconnected"}`}>
            {connectionError ? "主服务连接中断" : status ? "主服务已连接" : "正在连接主服务…"}
          </span>
          {status && <span className={`connection-pill ${status.bridge_connected && !connectionError ? "connected" : "disconnected"}`}>
            {connectionError ? "桌面执行端状态待确认" : status.bridge_connected ? "桌面执行端已连接" : "桌面执行端未连接"}
          </span>}
        </div>
      </section>

      {(connectionError || actionError) && <div className="error-banner" role="alert">
        {connectionError && <p>{connectionError}</p>}
        {actionError && <p>{actionError}</p>}
      </div>}

      <div className="workspace-columns">
        <section className="panel" aria-labelledby="speech-heading">
          <p className="eyebrow">人工控制</p>
          <h2 id="speech-heading">文字播报</h2>
          <p className="muted">输入文字，交给桌面执行端播报。</p>
          <form aria-labelledby="speech-heading" onSubmit={(event) => { void submit(event); }}>
            <label htmlFor="speech-text">播报文本</label>
            <textarea id="speech-text" value={text} onChange={(event) => setText(event.target.value)} rows={7}
              placeholder="写下想对直播间说的话…" disabled={pendingAction === "speech"}
              aria-describedby="speech-length" aria-invalid={textLength > 500} />
            <p id="speech-length" className={textLength > 500 ? "field-error" : "field-hint"}>{textLength} / 500 字{ textLength > 500 ? "，最多 500 字" : ""}</p>
            <label htmlFor="speech-voice">声音 ID</label>
            <input id="speech-voice" value={voiceId} onChange={(event) => setVoiceId(event.target.value)}
              disabled={pendingAction === "speech"} aria-describedby="voice-hint" aria-invalid={!validVoice} />
            <p id="voice-hint" className={validVoice ? "field-hint" : "field-error"}>使用已配置的声音；1–64 位字母、数字、下划线或连字符。</p>
            {status && !status.bridge_connected && !connectionError && <p className="availability-note">请先启动桌面执行客户端，连接后即可加入播报队列。</p>}
            <div className="form-actions">
              <button type="submit" className="primary-button" disabled={!canSubmit}>{pendingAction === "speech" ? "正在提交…" : "加入播报队列"}</button>
              <button type="button" className="stop-button" disabled={!canStop} onClick={() => { void controller.stop(); }}>{pendingAction === "stop" ? "正在停止…" : "停止全部播报"}</button>
            </div>
          </form>
        </section>

        <section className="panel history-panel" aria-labelledby="history-heading">
          <div className="section-title">
            <h2 id="history-heading">播报记录</h2>
            <span className="field-hint">进行中与最近 50 条历史</span>
          </div>
          <p className="muted">状态随服务更新，播放结束后显示完成。</p>
          {connectionError && speeches.length > 0 && <p className="availability-note">当前展示上次连接时的记录。</p>}
          {speeches.length === 0 ? <div className="empty-state"><p>还没有播报任务</p><span>提交第一段文字，记录会显示在这里。</span></div> :
            <ol className="speech-list" aria-label="播报任务">
              {speeches.map((task) => <li key={task.id}>
                <div className="task-heading"><span className={`task-status task-${task.status}`}>{statusLabels[task.status]}</span><span className="field-hint">声音 {task.voice_id}</span></div>
                <p className="speech-text">{task.text}</p>
                {task.error && <p className="field-error">{task.error}</p>}
                {task.status === "unknown" && <p className="field-hint">执行端断线，无法确认播放结果；不会自动重播。</p>}
              </li>)}
            </ol>}
        </section>
      </div>
    </div>
  );
}
