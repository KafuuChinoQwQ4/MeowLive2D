import { createContext, useContext, useEffect, useId, useMemo, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import "./feedback.css";

export type OperationNotice = { kind: "success" | "error" | "info"; title: string; message: string; tips?: string[]; restoreFocus?: Element | null; scope?: string };
const fallback = {
  enabled: false,
  notify: (_notice: OperationNotice) => {},
  success: (_title: string, _message: string) => {},
  error: (_title: string, _error: unknown, _fallback?: string) => {},
  reportIssue: (_key: string, _title: string, _error: unknown) => {},
  clearIssue: (_key: string) => {},
};
const FeedbackContext = createContext(fallback);
const FeedbackScopeContext = createContext("global");
const InlineNoticesContext = createContext<Record<string, OperationNotice>>({});
export function useFeedback() {
  const feedback = useContext(FeedbackContext);
  const scope = useContext(FeedbackScopeContext);
  return useMemo(() => ({ ...feedback,
    notify: (notice: OperationNotice) => feedback.notify({ ...notice, scope }),
    success: (title: string, message: string) => feedback.notify({ kind: "success", title, message, scope }),
  }), [feedback, scope]);
}

export function FeedbackScope({ name, children }: { name: string; children: ReactNode }) {
  return <FeedbackScopeContext.Provider value={name}>{children}<InlineFeedback scope={name} /></FeedbackScopeContext.Provider>;
}

function InlineFeedback({ scope }: { scope: string }) {
  const notice = useContext(InlineNoticesContext)[scope];
  return notice ? <p className="operation-inline-feedback" role="status" aria-label={notice.title} aria-live="polite"><strong>{notice.title}</strong>：{notice.message}</p> : null;
}

function messageOf(error: unknown, fallbackMessage = "操作失败，请稍后重试。") {
  return error instanceof Error ? error.message : typeof error === "string" ? error : fallbackMessage;
}

export function FeedbackProvider({ children }: { children?: ReactNode }) {
  const [queue, setQueue] = useState<Array<OperationNotice & { id: number }>>([]);
  const [inlineNotices, setInlineNotices] = useState<Record<string, OperationNotice>>({});
  const sequence = useRef(0);
  const issues = useRef(new Set<string>());
  const lastControl = useRef<Element | null>(null);
  useEffect(() => {
    const remember = (event: FocusEvent) => {
      const target = event.target;
      if (target instanceof HTMLElement && target !== document.body && !target.closest("dialog")) lastControl.current = target;
    };
    document.addEventListener("focusin", remember);
    return () => document.removeEventListener("focusin", remember);
  }, []);
  const feedback = useMemo(() => {
    const notify = (notice: OperationNotice) => {
      if (notice.kind !== "error") {
        setInlineNotices(current => ({ ...current, [notice.scope ?? "global"]: notice }));
        return;
      }
      const active = document.activeElement;
      const restoreFocus = notice.restoreFocus ?? (active !== document.body && !active?.closest("dialog") ? active : lastControl.current);
      setQueue(current => [...current, { ...notice, restoreFocus, id: ++sequence.current }]);
    };
    return {
      enabled: true, notify,
      success: (title: string, message: string) => notify({ kind: "success", title, message }),
      error: (title: string, error: unknown, fallbackMessage?: string) => notify({ kind: "error", title, message: messageOf(error, fallbackMessage) }),
      reportIssue: (key: string, title: string, error: unknown) => {
        if (issues.current.has(key)) return;
        issues.current.add(key);
        notify({ kind: "error", title, message: messageOf(error) });
      },
      clearIssue: (key: string) => { issues.current.delete(key); },
    };
  }, []);
  const notice = queue[0];
  return <FeedbackContext.Provider value={feedback}><InlineNoticesContext.Provider value={inlineNotices}>{children}
    <InlineFeedback scope="global" />
    {notice && <OperationResultDialog key={notice.id} notice={notice} onClose={() => setQueue(current => current.slice(1))} />}
  </InlineNoticesContext.Provider></FeedbackContext.Provider>;
}

export function OperationResultDialog({ notice, onClose }: { notice: OperationNotice; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const id = useId();
  useEffect(() => {
    const element = dialog.current!;
    const previous = notice.restoreFocus ?? document.activeElement;
    if (typeof element.showModal === "function") element.showModal();
    else element.setAttribute("open", "");
    closeButton.current?.focus();
    return () => {
      if (typeof element.close === "function") element.close();
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, []);
  return createPortal(<dialog ref={dialog} className={`operation-result-dialog is-${notice.kind}`} aria-labelledby={`${id}-title`} aria-describedby={`${id}-message`}
    onCancel={event => { event.preventDefault(); onClose(); }}>
    <div className="operation-result-heading"><span aria-hidden="true">{notice.kind === "error" ? "!" : notice.kind === "success" ? "✓" : "i"}</span><h2 id={`${id}-title`}>{notice.title}</h2></div>
    <p id={`${id}-message`} role={notice.kind === "error" ? "alert" : undefined}>{notice.message}</p>
    {!!notice.tips?.length && <aside className="operation-result-tips" aria-label="操作建议"><strong>Tips · {notice.kind === "error" ? "建议这样处理" : "接下来可以"}</strong><ul>{notice.tips.map(tip => <li key={tip}>{tip}</li>)}</ul></aside>}
    <div className="operation-result-actions"><button ref={closeButton} type="button" onClick={onClose}>知道了</button></div>
  </dialog>, document.body);
}
