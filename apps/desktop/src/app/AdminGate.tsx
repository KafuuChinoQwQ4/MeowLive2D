import { useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import type { AdminSessionClient, SessionStatus } from "../services/server/auth";

type GateState = SessionStatus | "loading" | "unavailable";
const STATUS_TIMEOUT_MS = 5_000;

export function AdminGate({ client, children }: { client: AdminSessionClient; children: ReactNode }) {
  const [state, setState] = useState<GateState>("loading");
  const [attempt, setAttempt] = useState(0);
  const [credential, setCredential] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const statusEpoch = useRef(0);
  const operationEpoch = useRef(0);

  useEffect(() => {
    let active = true;
    const epoch = ++statusEpoch.current;
    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), STATUS_TIMEOUT_MS);
    const unsubscribe = client.subscribe(authenticated => {
      if (active) {
        ++statusEpoch.current;
        ++operationEpoch.current;
        setPending(false);
        setCredential("");
        setError(null);
        setState({ enabled: true, authenticated });
      }
    });
    setState("loading");
    setPending(false);
    setError(null);
    setCredential("");
    void client.status(controller.signal)
      .then(status => {
        if (active && epoch === statusEpoch.current) setState(status);
      })
      .catch(() => {
        if (active && epoch === statusEpoch.current) setState("unavailable");
      })
      .finally(() => window.clearTimeout(timeout));
    return () => {
      active = false;
      ++statusEpoch.current;
      ++operationEpoch.current;
      controller.abort();
      window.clearTimeout(timeout);
      unsubscribe();
    };
  }, [attempt, client]);

  async function login(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!credential.trim() || pending) return;
    const epoch = ++operationEpoch.current;
    setPending(true);
    setError(null);
    try {
      await client.login(credential);
    } catch {
      if (epoch === operationEpoch.current) setError("登录失败，请检查管理员口令后重试。");
    } finally {
      if (epoch === operationEpoch.current) {
        setCredential("");
        setPending(false);
      }
    }
  }

  async function logout() {
    const epoch = ++operationEpoch.current;
    setPending(true);
    try {
      await client.logout();
    } catch {
      if (epoch === operationEpoch.current) setError("退出会话时无法连接主服务，已在本窗口清除会话。");
    } finally {
      if (epoch === operationEpoch.current) {
        setCredential("");
        setPending(false);
      }
    }
  }

  if (state === "loading") return <p className="availability-note" role="status">正在连接控制面板服务…</p>;
  if (state === "unavailable") return <div className="error-banner" role="alert">
    无法连接控制面板服务，请确认主服务已启动。
    <button type="button" className="secondary-button" onClick={() => setAttempt(value => value + 1)}>重新核对</button>
  </div>;
  if (!state.enabled) return <>{children}</>;
  if (!state.authenticated) {
    return <section className="panel admin-login" aria-labelledby="admin-login-heading">
      <p className="eyebrow">管理员访问</p>
      <h2 id="admin-login-heading">登录后查看此页面</h2>
      <form onSubmit={login}>
        <label>管理员口令<input aria-label="管理员口令" type="password" autoComplete="current-password" value={credential} onChange={event => setCredential(event.target.value)} disabled={pending} /></label>
        <button className="primary-button" type="submit" disabled={pending || !credential.trim()}>{pending ? "正在登录…" : "登录管理员"}</button>
      </form>
      {error && <p role="alert">{error}</p>}
    </section>;
  }
  return <>
    <div className="admin-session-actions"><button type="button" className="secondary-button" onClick={() => void logout()} disabled={pending}>{pending ? "正在退出…" : "退出管理员"}</button></div>
    {error && <p className="error-banner" role="alert">{error}</p>}
    {children}
  </>;
}
