import { useState } from "react";
import type { FormEvent } from "react";
import type { EventBatchRequest, EventBatchResult, EventPayload, LiveEventInput } from "@meowlive/contracts";
import { useFeedback } from "../../app/feedback/OperationFeedback";
import { eventPayloadError } from "../../services/server/eventPayload";

type EventMode = EventPayload["type"];

function readReplay(value: string): { events?: LiveEventInput[]; error?: string } {
  let parsed: unknown;
  try {
    parsed = JSON.parse(value);
  } catch {
    return { error: "回放内容不是有效的 JSON。" };
  }
  if (!Array.isArray(parsed)) return { error: "回放内容必须是 LiveEventInput JSON 数组。" };
  if (parsed.length < 1 || parsed.length > 100) return { error: "每批回放必须包含 1 到 100 条事件。" };
  for (const item of parsed) {
    if (typeof item !== "object" || item === null) return { error: "每条回放事件必须是对象。" };
    const event = item as Record<string, unknown>;
    if (typeof event.id !== "string" || !event.id.trim()
      || typeof event.source !== "string" || !event.source.trim()
      || typeof event.viewer !== "string" || !event.viewer.trim()) {
      return { error: "事件 ID、来源和观众名称不能为空。" };
    }
    const error = eventPayloadError(event.kind);
    if (error) return { error };
    if (event.gift_metadata !== undefined && event.gift_metadata !== null
      && (event.kind as Record<string, unknown>).type !== "gift") {
      return { error: "礼物元数据只能附加在 gift 事件上。" };
    }
  }
  return { events: parsed as LiveEventInput[] };
}

function resultMessage(result: EventBatchResult): string {
  if (result.persisted !== undefined) {
    const suffix = result.unscheduled && result.unscheduled > 0
      ? `，${result.unscheduled} 条未安排回应`
      : "";
    return `已持久保存 ${result.persisted} 条事件${suffix}。`;
  }
  if (result.accepted === 0 && result.duplicates > 0) return `${result.duplicates} 条事件已存在，未重复接收。`;
  if (result.duplicates > 0) return `已接收 ${result.accepted} 条事件，忽略 ${result.duplicates} 条重复事件。`;
  return `已接收 ${result.accepted} 条事件。`;
}

export function EventSimulator({ disabled, onSubmit }: {
  disabled: boolean;
  onSubmit: (batch: EventBatchRequest) => Promise<EventBatchResult | null>;
}) {
  const notices = useFeedback();
  const [mode, setMode] = useState<EventMode>("chat");
  const [viewer, setViewer] = useState("");
  const [chatText, setChatText] = useState("");
  const [giftName, setGiftName] = useState("");
  const [giftCount, setGiftCount] = useState("1");
  const [superChatText, setSuperChatText] = useState("");
  const [superChatAmount, setSuperChatAmount] = useState("30");
  const [superChatSeconds, setSuperChatSeconds] = useState("60");
  const [replay, setReplay] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);

  async function submitSimulated(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const now = Date.now();
    let kind: EventPayload;
    switch (mode) {
      case "chat": kind = { type: "chat", text: chatText.trim() }; break;
      case "gift": kind = { type: "gift", name: giftName.trim(), count: Number(giftCount) }; break;
      case "super_chat": kind = { type: "super_chat", text: superChatText.trim(), amount_cny: Number(superChatAmount), start_at_ms: now, end_at_ms: now + Number(superChatSeconds) * 1_000 }; break;
      case "room_enter": kind = { type: "room_enter" }; break;
    }
    let error = !viewer.trim() ? "观众名称不能为空。" : eventPayloadError(kind);
    if (mode === "super_chat" && (!Number.isSafeInteger(Number(superChatSeconds)) || Number(superChatSeconds) < 1)) {
      error = "SC 有效时长必须是正整数秒。";
    }
    if (error) {
      setValidationError(error);
      notices.error("模拟事件检查失败", error);
      return;
    }
    setValidationError(null);
    setFeedback(null);
    const result = await onSubmit({ events: [{ id: crypto.randomUUID(), source: "simulator", viewer: viewer.trim(), kind }] });
    if (!result) return;
    setFeedback(resultMessage(result));
    if (mode === "chat") setChatText("");
    if (mode === "gift") setGiftName("");
    if (mode === "super_chat") setSuperChatText("");
  }

  async function submitReplay(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const decoded = readReplay(replay);
    if (!decoded.events) {
      setValidationError(decoded.error ?? "回放内容无效。");
      notices.error("事件回放检查失败", decoded.error ?? "回放内容无效。");
      return;
    }
    setValidationError(null);
    setFeedback(null);
    const result = await onSubmit({ events: decoded.events });
    if (!result) return;
    setFeedback(resultMessage(result));
    setReplay("");
  }

  return (
    <section className="panel event-simulator" aria-labelledby="event-simulator-heading">
      <h2 id="event-simulator-heading">直播事件</h2>
      <p className="muted">这里发送模拟事件，用于检查播报和互动策略。SC 金额单位为人民币元。</p>
      <div className="segmented-control" aria-label="模拟事件类型">
        <button type="button" aria-pressed={mode === "chat"} onClick={() => setMode("chat")}>聊天</button>
        <button type="button" aria-pressed={mode === "gift"} onClick={() => setMode("gift")}>礼物</button>
        <button type="button" aria-pressed={mode === "super_chat"} onClick={() => setMode("super_chat")}>SC</button>
        <button type="button" aria-pressed={mode === "room_enter"} onClick={() => setMode("room_enter")}>进房</button>
      </div>
      <form noValidate onSubmit={(event) => { void submitSimulated(event); }}>
        <label htmlFor="event-viewer">观众名称</label>
        <input id="event-viewer" value={viewer} disabled={disabled} onChange={(event) => setViewer(event.target.value)} />
        {mode === "chat" && <>
          <label htmlFor="event-chat">聊天内容</label>
          <textarea id="event-chat" rows={4} value={chatText} disabled={disabled} onChange={(event) => setChatText(event.target.value)} />
        </>}
        {mode === "gift" && <div className="compact-fields">
          <div><label htmlFor="event-gift-name">礼物名称</label><input id="event-gift-name" value={giftName} disabled={disabled} onChange={(event) => setGiftName(event.target.value)} /></div>
          <div><label htmlFor="event-gift-count">礼物数量</label><input id="event-gift-count" type="number" min="1" step="1" value={giftCount} disabled={disabled} onChange={(event) => setGiftCount(event.target.value)} /></div>
        </div>}
        {mode === "super_chat" && <>
          <label htmlFor="event-super-chat">SC 内容</label>
          <textarea id="event-super-chat" rows={4} value={superChatText} disabled={disabled} onChange={(event) => setSuperChatText(event.target.value)} />
          <div className="compact-fields">
            <div><label htmlFor="event-super-chat-amount">SC 金额（元）</label>
              <input id="event-super-chat-amount" type="number" min="1" max="1000000" step="1" value={superChatAmount} disabled={disabled} onChange={(event) => setSuperChatAmount(event.target.value)} /></div>
            <div><label htmlFor="event-super-chat-duration">SC 有效时长（秒）</label>
              <input id="event-super-chat-duration" type="number" min="1" step="1" value={superChatSeconds} disabled={disabled} onChange={(event) => setSuperChatSeconds(event.target.value)} /></div>
          </div>
        </>}
        {mode === "room_enter" && <p className="field-hint">进房事件携带当前昵称；是否欢迎由 Agent 的互动策略决定。</p>}
        <div className="form-actions"><button className="primary-button" type="submit" disabled={disabled}>发送模拟事件</button></div>
      </form>

      <div className="panel-divider" />
      <form onSubmit={(event) => { void submitReplay(event); }}>
        <h3>事件回放</h3>
        <label htmlFor="event-replay">事件回放 JSON</label>
        <textarea id="event-replay" rows={6} value={replay} disabled={disabled} onChange={(event) => setReplay(event.target.value)}
          placeholder='[{"id":"event-1","source":"platform","viewer":"观众","kind":{"type":"chat","text":"你好"}}]' />
        <div className="form-actions"><button type="submit" disabled={disabled}>提交事件回放</button></div>
      </form>
      {validationError && <p className="field-error" role="alert">{validationError}</p>}
      {feedback && <p className="success-banner" role="status">{feedback}</p>}
    </section>
  );
}
