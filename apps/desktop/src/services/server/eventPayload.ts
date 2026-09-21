import type { EventPayload } from "@meowlive/contracts";

function boundedText(value: unknown, max: number): boolean {
  return typeof value === "string" && value.trim().length > 0 && [...value].length <= max;
}

function boundedInteger(value: unknown, min: number, max: number): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= min && value <= max;
}

export function eventPayloadError(value: unknown): string | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return "事件载荷无效。";
  const kind = value as Record<string, unknown>;
  switch (kind.type) {
    case "chat":
      return boundedText(kind.text, 500) ? null : "聊天内容须为 1 到 500 个字符，不能为空。";
    case "gift":
      if (!boundedText(kind.name, 100)) return "礼物名称须为 1 到 100 个字符，不能为空。";
      return boundedInteger(kind.count, 1, 10000) ? null : "礼物数量必须是 1 到 10000 的正整数。";
    case "super_chat":
      if (!boundedText(kind.text, 500)) return "SC 内容须为 1 到 500 个字符，不能为空。";
      if (!boundedInteger(kind.amount_cny, 1, 1_000_000)) return "SC 金额必须是 1 到 1000000 元的整数。";
      if (!boundedInteger(kind.start_at_ms, 0, Number.MAX_SAFE_INTEGER)
        || !boundedInteger(kind.end_at_ms, 0, Number.MAX_SAFE_INTEGER)
        || kind.end_at_ms <= kind.start_at_ms) return "SC 有效期须使用非负整数毫秒时间戳，且结束时间晚于开始时间。";
      return null;
    case "room_enter":
      return null;
    default:
      return "事件类型必须是 chat、gift、super_chat 或 room_enter。";
  }
}

export function isEventPayload(value: unknown): value is EventPayload {
  return eventPayloadError(value) === null;
}
