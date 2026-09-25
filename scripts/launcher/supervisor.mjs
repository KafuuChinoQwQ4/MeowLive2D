import { spawn } from 'node:child_process';
import { mkdir } from 'node:fs/promises';
import { dirname } from 'node:path';
import { randomBytes } from 'node:crypto';
import { setTimeout as wait } from 'node:timers/promises';
import { inspectEndpoint } from './health.mjs';
import { captureOutput } from './log.mjs';
import { displayPath } from './paths.mjs';
import { createMemoryReader, memoryPressure } from './memory.mjs';

export class Supervisor {
  constructor({ definitions, setup, startupMs = 300_000, stopMs = 8000, pollMs = 1500, readMemory = createMemoryReader() }) {
    this.setup = setup;
    this.token = randomBytes(32).toString('hex');
    this.startupMs = startupMs;
    this.stopMs = stopMs;
    this.closed = false;
    this.readMemory = readMemory;
    this.records = new Map(definitions.map(def => [def.id, { def, state: 'stopped', message: def.issue ?? '尚未启动。',
      child: null, occupied: false, queue: Promise.resolve(), completion: Promise.resolve(), timer: null }]));
    this.timer = setInterval(() => void this.refresh(), pollMs);
    this.timer.unref();
  }

  async locked(record, work) {
    const result = record.queue.then(work);
    record.queue = result.catch(() => {});
    return result;
  }

  async initialize() {
    this.initializing ??= (async () => {
      try {
        await this.setEnabled('server', true);
        await this.waitUntilReady('server');
      } catch (error) {
        this.startFailed('server', error);
        return;
      }
      if (this.records.has('tts')) {
        try { await this.setEnabled('tts', true); }
        catch (error) { this.startFailed('tts', error); }
      }
    })();
    await this.initializing;
    return this;
  }

  startFailed(id, error) {
    const record = this.records.get(id);
    if (record && !this.closed && !record.child) { record.state = 'failed'; record.message = error.message; }
  }

  async waitUntilReady(id) {
    while (!this.closed) {
      await this.refresh();
      const record = this.records.get(id);
      if (['running', 'external'].includes(record.state)) return;
      if (record.state !== 'starting') throw new Error(record.message);
      await wait(100);
    }
    throw new Error('控制面板正在退出。');
  }

  async refresh() {
    if (this.closed || this.refreshing) return this.refreshing;
    this.refreshing = Promise.all([...this.records.values()].filter(record => !record.reconfiguring).map(record => this.locked(record, async () => {
      if (this.closed || record.state === 'stopping' || record.def.issue) return;
      const child = record.child;
      if (child && record.def.memoryGuard) {
        try {
          const issue = memoryPressure(await this.readMemory(), false);
          if (record.child !== child || this.closed) return;
          if (issue) { this.terminate(record, 'failed', issue); return; }
        } catch {
          if (record.child !== child || this.closed) return;
          if (record.state === 'starting' && Date.now() - record.startedAt > this.startupMs) {
            this.terminate(record, 'failed', '启动超时且无法读取系统内存，已停止 TTS；请检查 WSL 的 Windows 互操作。');
            return;
          }
          record.message = '暂时无法读取系统内存，请留意 Windows 任务管理器；可关闭 TTS 释放资源。';
          return;
        }
      }
      const health = await inspectEndpoint(record.def);
      if (record.child !== child || this.closed) return;
      record.occupied = health.occupied;
      if (child) {
        if (health.healthy) { record.state = 'running'; record.message = '服务已就绪。'; }
        else if (record.state === 'starting' && Date.now() - record.startedAt > this.startupMs) {
          this.terminate(record, 'failed', '启动超时，已取消本次启动。请查看运行日志后重试。');
        } else if (record.state === 'running') record.message = '进程仍在运行，暂未收到状态响应；可能正在处理任务。';
      } else if (health.healthy) { record.state = 'external'; record.message = '已由其他终端启动。若要关闭，请返回原终端操作。'; }
      else if (health.occupied) { record.state = 'failed'; record.message = '端口被占用，但未检测到匹配的服务。请检查原终端或端口配置。'; }
      else if (record.state === 'external') { record.state = 'stopped'; record.message = '外部服务已停止，可以在此启动。'; }
    }))).finally(() => { this.refreshing = null; });
    return this.refreshing;
  }

  async snapshot() {
    await this.refresh();
    return { schema_version: 1, session_token: this.token, setup: this.setup,
      services: [...this.records.values()].map(record => ({ id: record.def.id, state: record.state,
        managed: Boolean(record.child), message: record.def.issue ?? record.message, url: record.def.url,
        log_path: displayPath(record.def.logPath, record.def.cwd, record.def.env?.HOME),
        can_start: !this.closed && !record.reconfiguring && !record.def.issue && !record.child && !record.occupied,
        can_stop: !record.reconfiguring && Boolean(record.child) && record.state !== 'stopping' })) };
  }

  async setEnabled(id, enabled) {
    const record = this.records.get(id);
    if (!record || typeof enabled !== 'boolean') throw new Error('未知服务或无效开关值。');
    return this.locked(record, () => this.changeEnabled(record, enabled));
  }

  async changeEnabled(record, enabled) {
    if (this.closed) throw new Error('控制面板正在退出。');
    if (!enabled) {
      if (record.state === 'external') throw new Error('此服务由其他终端启动，请回原终端停止。');
      if (record.child && record.state !== 'stopping') this.terminate(record, 'stopped', '服务已停止。');
      return;
    }
    if (record.state === 'stopping') throw new Error('正在停止，请等待完成后再启动。');
    if (record.child || record.state === 'external') return;
    if (record.def.issue) throw new Error(record.def.issue);
    const health = await inspectEndpoint(record.def);
    record.occupied = health.occupied;
    if (health.healthy) { record.state = 'external'; record.message = '已由其他终端启动，请回原终端管理。'; return; }
    if (health.occupied) { record.state = 'failed'; record.message = '服务端口已被占用，请检查端口或原终端。'; throw new Error(record.message); }
    if (record.def.memoryGuard) {
      let issue;
      try { issue = memoryPressure(await this.readMemory(), true); }
      catch { issue = '无法读取系统内存，暂不启动 TTS。请检查 WSL 的 Windows 互操作后重试。'; }
      if (issue) { record.state = 'failed'; record.message = issue; throw new Error(issue); }
    }
    if (this.closed) throw new Error('控制面板正在退出。');
    await mkdir(dirname(record.def.logPath), { recursive: true, mode: 0o700 });
    const child = spawn(record.def.command, record.def.args, { cwd: record.def.cwd, env: record.def.env,
      detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
    record.child = child;
    record.startedAt = Date.now();
    record.state = 'starting';
    record.message = record.def.id === 'tts' ? '正在启动 TTS 并自动加载所选语音模型，请稍候。' : '正在检查编译并启动主服务，请稍候。';
    record.finalState = null;
    record.diagnostic = null;
    captureOutput(child, record.def, value => { record.diagnostic = value; });
    child.once('error', error => {
      record.diagnostic = error.code === 'ENOENT' ? '未找到启动命令，请检查 Rust / Python 是否安装并可执行。' : '无法创建服务进程，请检查本机执行权限。';
    });
    record.completion = new Promise(resolve => child.once('close', (code, signal) => {
      clearTimeout(record.timer);
      if (record.child === child) {
        record.child = null;
        record.occupied = false;
        record.state = record.finalState ?? 'failed';
        record.message = record.finalMessage ?? record.diagnostic ?? `服务进程退出（${code ?? signal ?? '未知原因'}），请查看运行日志后重试。`;
      }
      resolve();
    }));
    record.finalMessage = null;
  }

  async configureTts(update, { restart = true } = {}) {
    const record = this.records.get('tts');
    // Mark the queued operation before yielding so status polls never queue behind it.
    record.reconfiguring = (record.reconfiguring ?? 0) + 1;
    return this.locked(record, async () => {
      if (this.closed) throw new Error('控制面板正在退出。');
      const health = await inspectEndpoint(record.def);
      if (!record.child && health.occupied) throw new Error('请先关闭 TTS 的外部服务，再切换语音模型；请回原终端关闭。');
      const resumePrevious = Boolean(record.child);
      if (record.child) {
        if (record.state !== 'stopping') this.terminate(record, 'stopped', '旧模型已停止，正在切换语音模型…');
        record.message = '正在暂停 TTS 并释放旧模型，切换完成后自动启动…';
        await record.completion;
      }
      if (this.closed) throw new Error('控制面板正在退出。');
      try { record.def = await update(record.def); }
      catch (error) {
        // A failed configuration write must not strand a previously running model.
        if (resumePrevious && !this.closed) {
          try { await this.changeEnabled(record, true); }
          catch (restartError) { this.startFailed('tts', restartError); }
        }
        throw error;
      }
      record.state = 'stopped'; record.occupied = false;
      record.message = record.def.issue ?? '模型已选择，TTS 将自动启动。';
      if (restart) {
        try { await this.changeEnabled(record, true); }
        catch (error) { this.startFailed('tts', error); throw error; }
      }
    }).finally(() => { record.reconfiguring -= 1; });
  }

  terminate(record, finalState, finalMessage) {
    const child = record.child;
    if (!child) return;
    record.state = 'stopping';
    record.message = '正在停止并释放资源…';
    record.finalState = finalState;
    record.finalMessage = finalMessage;
    const signal = name => {
      if (record.child !== child || !child.pid) return;
      try { process.kill(-child.pid, name); }
      catch (error) { if (error.code !== 'ESRCH') record.diagnostic = '无法停止进程，请检查本机进程权限。'; }
    };
    signal('SIGINT');
    record.timer = setTimeout(() => signal('SIGKILL'), this.stopMs);
    record.timer.unref();
  }

  async close() {
    this.closed = true;
    clearInterval(this.timer);
    await Promise.all([...this.records.values()].map(record => this.locked(record, async () => {
      if (record.child && record.state !== 'stopping') this.terminate(record, 'stopped', '控制面板退出，服务已停止。');
      await record.completion;
    })));
  }
}
