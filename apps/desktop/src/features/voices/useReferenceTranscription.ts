import { useEffect, useRef, useState } from "react";
import type { TrainingClient } from "../../services/server/training";

/** 将本地转写绑定到当前参考音频；保留手工编辑，丢弃过期请求。 */
export function useReferenceTranscription(audio: File | null, language: string, client: Pick<TrainingClient, "transcribe">) {
  const [text, setValue] = useState("");
  const value = useRef("");
  const generated = useRef<string | null>(null);
  const request = useRef<AbortController | null>(null);
  const [transcribing, setTranscribing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const supported = ["zh", "en", "ja", "ko", "yue"].includes(language);

  function setText(next: string) {
    request.current?.abort();
    generated.current = null;
    value.current = next;
    setValue(next);
    setError(null);
    setTranscribing(false);
  }

  useEffect(() => {
    if (generated.current !== null && value.current === generated.current) {
      value.current = "";
      setValue("");
    }
    generated.current = null;
    setError(null);
    setTranscribing(false);
    if (!audio || !supported || value.current.trim()) return;
    const controller = new AbortController();
    request.current = controller;
    setTranscribing(true);
    void client.transcribe(audio, language, controller.signal).then(result => {
      if (controller.signal.aborted || value.current.trim()) return;
      value.current = result.text;
      generated.current = result.text;
      setValue(result.text);
    }).catch((cause: unknown) => {
      if (!controller.signal.aborted) setError(cause instanceof Error ? cause.message : "无法提取参考文本");
    }).finally(() => {
      if (!controller.signal.aborted) setTranscribing(false);
      if (request.current === controller) request.current = null;
    });
    return () => controller.abort();
  }, [audio, language, supported, client, attempt]);

  return { text, setText, transcribing, error, supported, retry: () => setAttempt(previous => previous + 1) };
}
