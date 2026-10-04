import { useEffect, useRef, useState } from "react";
import { updateClient, type UpdateClient, type UpdateStatus } from "../../services/updates";
const busyPhases = new Set(["checking", "downloading", "installing"]);
function size(bytes: number) { return `${(bytes / 1024 / 1024).toFixed(1)} MiB`; }
export function AppUpdatePanel({ client = updateClient, disabled = false, pollIntervalMs = 1000 }: { client?: UpdateClient; disabled?: boolean; pollIntervalMs?: number }) {
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [error, setError] = useState("");
  const [pending, setPending] = useState(false);
  const mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const refresh = async () => {
      try { const value = await client.status(); if (!stopped) setStatus(value); }
      catch (reason) { if (!stopped) setError(String(reason)); }
      finally { if (!stopped) timer = setTimeout(() => { void refresh(); }, pollIntervalMs); }
    };
    void (async () => {
      try {
        const value = await client.status();
        if (stopped) return;
        setStatus(value);
        if (!busyPhases.has(value.phase) && value.phase !== "ready") await client.check();
      } catch (reason) { if (!stopped) setError(String(reason)); }
      if (!stopped) await refresh();
    })();
    return () => { stopped = true; mounted.current = false; if (timer) clearTimeout(timer); };
  }, [client, pollIntervalMs]);
  const action = async (name: "check" | "prepare" | "cancel" | "install") => {
    if (pending) return;
    setPending(true); setError("");
    try {
      await client[name]();
      const value = await client.status();
      if (mounted.current) setStatus(value);
    } catch (reason) { if (mounted.current) setError(String(reason)); }
    finally { if (mounted.current) setPending(false); }
  };
  const busy = pending || Boolean(status && busyPhases.has(status.phase));
  return <section className="connection-card" aria-labelledby="app-update-title">
    <h2 id="app-update-title">App 更新</h2>
    <p>通过 GitHub Releases 获取经签名验证的 Windows 更新。</p>
    <p>当前版本：{status?.current_tag ?? "读取中…"}{status?.available_tag && ` · 可用版本：${status.available_tag}`}</p>
    <p role="status">{status?.message ?? "正在读取更新状态…"}</p>
    {error && <p role="alert">{error}</p>}
    {status && status.total_bytes > 0 && <>
      <progress aria-label="更新准备进度" max={status.total_bytes} value={Math.min(status.total_bytes, status.downloaded_bytes + status.reused_bytes)} />
      <p>安装包 {size(status.total_bytes)} · 已下载 {size(status.downloaded_bytes)} · 已复用 {size(status.reused_bytes)}</p>
      <p>{status.reused_bytes > 0 ? "增量更新：复用已验证的本地数据。" : "首次更新或缓存不可用时需要完整下载。"}</p>
    </>}
    {status?.release_notes && <details><summary>版本说明</summary><p style={{ whiteSpace: "pre-wrap" }}>{status.release_notes}</p></details>}
    <div className="button-row">
      <button type="button" disabled={busy || disabled} onClick={() => void action("check")}>检查更新</button>
      {status?.phase === "available" && <button type="button" disabled={busy || disabled} onClick={() => void action("prepare")}>下载更新</button>}
      {status?.phase === "ready" && <button type="button" disabled={busy || disabled} onClick={() => void action("install")}>安装并重启</button>}
      {status && ["checking", "downloading"].includes(status.phase) && <button type="button" disabled={pending} onClick={() => void action("cancel")}>取消更新</button>}
      <a href={status?.release_url ?? "https://github.com/KafuuChinoQwQ4/MeowLive2D/releases"} target="_blank" rel="noreferrer">查看 GitHub Releases</a>
    </div>
    <p>安装会停止受管服务并关闭 App；模型与用户配置保留。训练或环境安装期间不能更新。</p>
  </section>;
}
