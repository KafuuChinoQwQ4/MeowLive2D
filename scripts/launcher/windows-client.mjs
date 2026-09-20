import { access, mkdir, open, readFile, writeFile, rename } from 'node:fs/promises';
import { constants } from 'node:fs';
import { join, resolve, delimiter } from 'node:path';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { randomUUID } from 'node:crypto';
import { displayPath } from './paths.mjs';
import { enableWindowsVts } from './windows-config.mjs';

const execute = promisify(execFile);
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
async function exists(path) { try { await access(path, constants.R_OK); return true; } catch { return false; } }

async function clientFailure(path) {
  let file;
  try {
    file = await open(join(path, 'client.log'), 'r');
    const size = (await file.stat()).size;
    const buffer = Buffer.alloc(8192);
    const { bytesRead } = await file.read(buffer, 0, buffer.length, Math.max(0, size - buffer.length));
    if (/memory allocation of \d+ bytes failed|out of memory/i.test(buffer.toString('utf8', 0, bytesRead))) {
      return 'Windows 系统内存不足，执行端已退出。请关闭本地大模型或其他高占用程序，再重新连接；TTS 默认使用低内存模式。';
    }
  } catch { /* The helper may fail before creating a log. */ }
  finally { await file?.close().catch(() => {}); }
  return 'Windows 执行端启动失败，请展开日志位置查看原因，并确认默认扬声器可用。';
}

export async function inspectBridge(url) {
  try {
    const response = await fetch(`${url}/api/health`, { signal: AbortSignal.timeout(800), redirect: 'error' });
    if (!response.ok) { await response.body?.cancel(); return { ready: false, connected: false }; }
    let size = 0; const chunks = [];
    for await (const chunk of response.body) { size += chunk.length; if (size > 512 * 1024) throw new Error('oversize'); chunks.push(chunk); }
    const data = JSON.parse(Buffer.concat(chunks).toString());
    const ready = data.protocol_version === 3 && data.service === 'meowlive' && typeof data.bridge_connected === 'boolean';
    return { ready, connected: ready && data.bridge_connected };
  } catch { return { ready: false, connected: false }; }
}

export class WindowsClientSupervisor {
  constructor(configuration, { probe = inspectBridge, spawnHelper = spawn, translate, startupMs = 60_000, stopMs = 18_000, pollMs = 1500 } = {}) {
    this.root = configuration.modelSettings.root;
    this.environment = configuration.modelSettings.environment;
    this.url = configuration.definitions[0].url;
    this.paths = configuration.windowsSettings ?? {
      executable: join(this.root, 'target/windows-client/meowlive-client.exe'),
      config: join(this.root, 'target/windows-client/desktop.local.toml'),
    };
    this.issue = configuration.definitions[0].issue ? '请先解决主服务的启动配置问题。' : null;
    this.probe = probe; this.spawnHelper = spawnHelper;
    this.translate = translate ?? (async path => (await execute('wslpath', ['-w', path], { timeout: 3000, maxBuffer: 8192 })).stdout.trim());
    this.startupMs = startupMs; this.stopMs = stopMs;
    this.state = 'stopped'; this.message = '打开开关后自动启动 Windows 执行端并连接主服务。';
    this.queue = Promise.resolve(); this.closed = false; this.session = null;
    this.timer = setInterval(() => void this.refresh().catch(() => {}), pollMs); this.timer.unref();
  }

  async initialize() {
    if (this.environment.kind !== 'wsl2') this.issue ??= '此开关需要在 Windows 的 WSL2 中运行面板。请双击 launchers/start-windows.cmd。';
    if (!await exists(this.paths.executable)) this.issue ??= '未找到 Windows 执行程序，请把已编译的 meowlive-client.exe 放入 target/windows-client 后重新打开面板。';
    if (!await exists(this.paths.config)) this.issue ??= '未找到 Windows 执行端配置，请将 desktop.local.toml 放入 target/windows-client 后重新打开面板。';
    const candidates = ['/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe',
      ...(process.env.PATH ?? '').split(delimiter).map(folder => join(folder, 'powershell.exe'))];
    for (const candidate of candidates) if (await exists(candidate)) { this.powershell = candidate; break; }
    if (!this.powershell) this.issue ??= '未检测到 Windows 启动能力。请双击 launchers/start-windows.cmd 重新打开控制面板。';
    return this;
  }

  locked(work) { const result = this.queue.then(work); this.queue = result.catch(() => {}); return result; }

  async pulse(session) {
    const path = join(session.path, 'lease');
    await writeFile(`${path}.tmp`, String(Date.now()), { mode: 0o600 }); await rename(`${path}.tmp`, path);
  }

  async refresh() { return this.locked(() => this.update()); }
  async update() {
    if (this.closed) return;
    const bridge = await this.probe(this.url); this.bridge = bridge;
    const session = this.session;
    if (!session) {
      if (bridge.connected) { this.state = 'external'; this.message = '已有执行端连接，当前开关不会接管其他方式启动的程序。'; }
      else if (this.state === 'external') { this.state = 'stopped'; this.message = '外部执行端已断开，可以使用开关启动。'; }
      return;
    }
    if (this.state === 'stopping') return;
    try { await this.pulse(session); }
    catch { await this.requestStop('failed', '无法更新 Windows 启动状态，已请求停止执行端。'); return; }
    let report;
    try {
      const raw = await readFile(join(session.path, 'status.json'), 'utf8');
      if (raw.length <= 8192) report = JSON.parse(raw);
    } catch { /* The helper has not published its first status yet. */ }
    if (report?.state === 'failed') {
      await this.requestStop('failed', await clientFailure(session.path)); return;
    }
    if (report?.state === 'stopped') {
      await this.requestStop('failed', 'Windows 执行端已退出，可重新打开开关连接。'); return;
    }
    if (report?.state === 'running' && bridge.connected) {
      this.state = 'running'; this.message = 'Windows 执行端已连接，声音将在 Windows 默认扬声器播放。';
    } else if (this.state === 'running') {
      this.message = bridge.ready ? 'Windows 程序仍在运行，正在重新连接主服务…' : '主服务已断开；恢复主服务后将自动重连。';
      this.state = 'starting'; session.started = Date.now();
    } else if (Date.now() - session.started > this.startupMs) {
      await this.requestStop('failed', 'Windows 连接超时，已取消本次启动。请双击 launchers/start-windows.cmd 重开面板，并确认 Windows 默认扬声器可用。');
    }
  }

  async snapshot() {
    await this.refresh();
    const managed = Boolean(this.session);
    return { id: 'windows', state: this.state, managed, url: this.url,
      message: this.issue ?? (!managed && !this.bridge?.ready && this.state === 'stopped' ? '先打开主服务开关，运行就绪后即可连接 Windows 执行端。' : this.message),
      log_path: displayPath(this.session?.path ?? join(this.root, 'data/windows-launcher'), this.root),
      can_start: !this.closed && !this.issue && !managed && this.state !== 'external' && Boolean(this.bridge?.ready),
      can_stop: managed && this.state !== 'stopping' };
  }

  async setEnabled(enabled) {
    return this.locked(async () => {
      if (this.closed) throw new Error('控制面板正在退出。');
      if (!enabled) {
        if (this.state === 'external') throw new Error('此执行端由其他方式启动，请先关闭原来的 Windows 执行端窗口。');
        await this.requestStop('stopped', 'Windows 执行端已断开。'); return;
      }
      if (this.session || this.state === 'external') return;
      this.lastExit = null;
      if (this.issue) throw new Error(this.issue);
      const bridge = await this.probe(this.url);
      if (!bridge.ready) throw new Error('请先打开主服务开关，等待运行就绪。');
      if (bridge.connected) { this.state = 'external'; this.message = '已有执行端连接，请使用当前执行端。'; return; }
      try { await enableWindowsVts(this.paths.config); }
      catch (error) {
        this.state = 'failed'; this.message = error.message;
        throw error;
      }
      const path = join(this.root, 'data/windows-launcher', randomUUID());
      await mkdir(path, { recursive: true, mode: 0o700 });
      const session = { path, started: Date.now(), stopped: false, completion: Promise.resolve() };
      this.session = session; this.state = 'starting'; this.message = '正在打开 Windows 执行端并连接主服务…';
      try {
        const [helper, sessionPath, executable, config] = await Promise.all([
          this.translate(join(this.root, 'launchers/windows-client.ps1')), this.translate(path),
          this.translate(this.paths.executable), this.translate(this.paths.config),
        ]);
        // Configuration content is never copied to the web response. Windows reads the existing file directly.
        await writeFile(join(path, 'job.json'), JSON.stringify({ schema_version: 1, executable, configuration: config }), { mode: 0o600 });
        await this.pulse(session);
        const child = this.spawnHelper(this.powershell, ['-NoLogo', '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', helper, '-SessionDirectory', sessionPath],
          { cwd: this.root, env: Object.fromEntries(Object.entries(process.env).filter(([name]) => !/KEY|TOKEN|PASSWORD|SECRET/i.test(name))), stdio: ['ignore', 'pipe', 'pipe'] });
        session.child = child;
        // The helper writes bounded diagnostics to status.json; do not relay arbitrary native console output to the browser.
        child.stdout?.resume(); child.stderr?.resume();
        child.once('error', () => { session.error = true; });
        session.completion = new Promise(resolve => child.once('close', () => {
          clearTimeout(session.forceTimer);
          if (this.session === session) {
            this.lastExit = session;
            this.session = null; this.state = session.finalState ?? 'failed';
            this.message = session.finalMessage ?? 'Windows 启动进程退出。请双击 launchers/start-windows.cmd 重新打开面板，并检查执行端配置和默认扬声器。';
          }
          if (session.finalMessage) { resolve(); return; }
          // The helper usually exits before the next status poll. Enrich this
          // failure from its log, but never overwrite a subsequent start/exit.
          void clientFailure(session.path).then(message => {
            if (this.lastExit === session && this.state === 'failed' && !this.session) this.message = message;
          }).finally(resolve);
        }));
      } catch {
        this.session = null; this.state = 'failed'; this.message = '无法访问 Windows 执行端，请双击 launchers/start-windows.cmd 重新打开面板。';
        throw new Error(this.message);
      }
    });
  }

  async requestStop(finalState, message) {
    const session = this.session; if (!session || session.stopped) return;
    session.stopped = true; session.finalState = finalState; session.finalMessage = message;
    this.state = 'stopping'; this.message = '正在断开 Windows 执行端…';
    try { await writeFile(join(session.path, 'stop'), 'stop', { mode: 0o600 }); } catch { /* Lease expiry is the independent fallback. */ }
    session.forceTimer = setTimeout(() => session.child?.kill('SIGKILL'), this.stopMs); session.forceTimer.unref();
  }

  async stopBeforeServer() {
    const completion = await this.locked(async () => { await this.requestStop('stopped', '主服务关闭，Windows 执行端已断开。'); return { done: this.session?.completion }; });
    await completion.done;
  }

  async close() {
    this.closed = true; clearInterval(this.timer);
    await this.stopBeforeServer();
  }
}

export function managedServices(supervisor, windows) {
  return { token: supervisor.token,
    async snapshot() { const [snapshot, client] = await Promise.all([supervisor.snapshot(), windows.snapshot()]); return { ...snapshot, services: [...snapshot.services, client] }; },
    async setEnabled(id, enabled) {
      if (id === 'windows') return windows.setEnabled(enabled);
      if (id === 'server' && !enabled) await windows.stopBeforeServer();
      return supervisor.setEnabled(id, enabled);
    },
  };
}
