import { useState } from "react";
import icon from "../../src-tauri/icons/icon.png";
import { controlWindow, hasNativeWindow } from "../services/desktop/window";

export function WindowChrome() {
  const [error, setError] = useState("");
  if (!hasNativeWindow()) return null;
  const act = (action: Parameters<typeof controlWindow>[0]) => {
    setError("");
    void controlWindow(action).catch(() => setError("窗口操作失败，请重试。"));
  };
  return <header className="native-chrome">
    <div className="native-drag-area" onPointerDown={event => {
      if (event.button === 0 && event.detail !== 2) act("startDragging");
    }} onDoubleClick={() => act("toggleMaximize")}>
      <img src={icon} alt="" draggable={false} /><span>MeowLive2D</span>
      {error && <span role="alert">{error}</span>}
    </div>
    <div className="native-window-actions">
      <button aria-label="最小化窗口" title="最小化" onClick={() => act("minimize")}><svg viewBox="0 0 16 16"><path d="M4 8h8" /></svg></button>
      <button aria-label="最大化或还原窗口" title="最大化 / 还原" onClick={() => act("toggleMaximize")}><svg viewBox="0 0 16 16"><rect x="4" y="4" width="8" height="8" rx="1" /></svg></button>
      <button aria-label="关闭窗口" title="关闭" onClick={() => act("close")}><svg viewBox="0 0 16 16"><path d="m4 4 8 8m0-8-8 8" /></svg></button>
    </div>
  </header>;
}
