import { useEffect, useId, useRef } from "react";
import { createPortal } from "react-dom";
import type { TrainingNotice } from "./trainingFeedback";

export function TrainingResultDialog({ notice, onClose }: { notice: TrainingNotice; onClose: () => void }) {
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
  return createPortal(<dialog ref={dialog} className={`training-result-dialog is-${notice.kind}`} aria-labelledby={`${id}-title`} aria-describedby={`${id}-message`}
    onCancel={event => { event.preventDefault(); onClose(); }}>
    <div className="training-result-heading"><span aria-hidden="true">{notice.kind === "error" ? "!" : notice.kind === "success" ? "✓" : "i"}</span>
      <h2 id={`${id}-title`}>{notice.title}</h2></div>
    <p id={`${id}-message`} role={notice.kind === "error" ? "alert" : undefined}>{notice.message}</p>
    {notice.tips.length > 0 && <aside className="training-result-tips" aria-label="操作建议"><strong>Tips · {notice.kind === "error" ? "建议这样处理" : "接下来可以"}</strong><ul>{notice.tips.map(tip => <li key={tip}>{tip}</li>)}</ul></aside>}
    <div className="training-result-actions"><button ref={closeButton} type="button" onClick={onClose}>知道了</button></div>
  </dialog>, document.body);
}
