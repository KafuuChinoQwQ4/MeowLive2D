import { PROTOCOL_VERSION } from "@meowlive/contracts";
import type { ErrorResponse, ServerStatus, SpeechSnapshot, SpeechStatus } from "@meowlive/contracts";

export class ServerRequestError extends Error {
  constructor(readonly code: string, message: string, readonly status?: number) {
    super(message);
    this.name = "ServerRequestError";
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

const speechStates: SpeechStatus[] = ["queued", "synthesizing", "ready", "playing", "completed", "cancelled", "failed", "unknown"];

function isSpeech(value: unknown): value is SpeechSnapshot {
  return isRecord(value)
    && typeof value.id === "string"
    && Number.isInteger(value.generation)
    && typeof value.text === "string"
    && typeof value.voice_id === "string"
    && speechStates.includes(value.status as SpeechStatus)
    && (value.error === null || typeof value.error === "string");
}

export function readSpeech(value: unknown): SpeechSnapshot {
  if (!isSpeech(value)) throw new ServerRequestError("invalid_response", "主服务返回了无效的播报任务。");
  return value;
}

export function readServerStatus(value: unknown): ServerStatus {
  if (!isRecord(value)
    || typeof value.protocol_version !== "number"
    || typeof value.session_id !== "string"
    || typeof value.bridge_connected !== "boolean"
    || !Number.isInteger(value.generation)
    || !Array.isArray(value.speeches)
    || !value.speeches.every(isSpeech)) {
    throw new ServerRequestError("invalid_response", "主服务返回了无效的状态。");
  }
  if (value.protocol_version !== PROTOCOL_VERSION) {
    throw new ServerRequestError("protocol_mismatch", "主服务协议版本不兼容，请使用配套版本的面板与主服务。");
  }
  return value as ServerStatus;
}

export function readServerError(value: unknown): ErrorResponse | null {
  return isRecord(value) && typeof value.code === "string" && typeof value.message === "string"
    ? { code: value.code, message: value.message }
    : null;
}
