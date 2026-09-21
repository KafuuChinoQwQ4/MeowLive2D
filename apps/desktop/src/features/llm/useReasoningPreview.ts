import { useEffect, useMemo, useState } from "react";
import type { LlmReasoningRequest, LlmReasoningResult, LlmSettings } from "@meowlive/contracts";
import type { LlmClient } from "../../services/server/llm";

type Preview = { request: LlmReasoningRequest; client: LlmClient; attempt: number; result: LlmReasoningResult | null; error: string };

export function useReasoningPreview(client: LlmClient, settings: LlmSettings | null) {
  const provider = settings?.provider.trim();
  const apiFormat = settings?.api_format;
  const model = settings?.model.trim();
  const effort = settings?.reasoning_effort;
  const maxTokens = settings?.max_tokens;
  const request = useMemo<LlmReasoningRequest | null>(() => provider && apiFormat && model && effort && maxTokens !== undefined
    && Number.isInteger(maxTokens) && maxTokens >= 64 && maxTokens <= 65536
    ? { provider, api_format: apiFormat, model, reasoning_effort: effort, max_tokens: maxTokens } : null,
  [provider, apiFormat, model, effort, maxTokens]);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    if (!request) return;
    const controller = new AbortController();
    const timer = setTimeout(() => {
      void client.previewReasoning(request, controller.signal).then(result => {
        if (!controller.signal.aborted) setPreview({ request, client, attempt, result, error: "" });
      }).catch(reason => {
        if (!controller.signal.aborted) setPreview({ request, client, attempt, result: null, error: reason instanceof Error ? reason.message : "推理强度预览失败，请重试。" });
      });
    }, 200);
    return () => { clearTimeout(timer); controller.abort(); };
  }, [request, client, attempt]);

  const current = preview?.request === request && preview.client === client && preview.attempt === attempt ? preview : null;
  return {
    pending: request !== null && current === null,
    result: current?.result ?? null,
    error: current?.error ?? "",
    retry: () => setAttempt(value => value + 1),
  };
}
