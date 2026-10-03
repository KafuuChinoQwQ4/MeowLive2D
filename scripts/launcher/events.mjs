import { appendFile, mkdir, open, rename, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import { randomUUID } from 'node:crypto';

const levels = new Set(['debug', 'info', 'warn', 'error']);
const categories = new Set(['launcher', 'server', 'tts', 'windows', 'asr', 'models']);
const summaries = Object.freeze({ service_started: '正在启动服务', service_ready: '服务已就绪', service_failed: '服务启动或运行失败，请检查运行环境', service_stopping: '正在停止服务并释放资源', service_exited: '服务进程已退出', service_external: '检测到外部启动的服务', model_selected: '本地模型已选择', model_download_started: '模型下载已开始', model_download_completed: '模型下载与校验已完成', model_download_failed: '模型下载或校验失败', model_download_cancelled: '模型下载已取消', model_scan_completed: '本地模型扫描已完成', output_stderr: '服务产生标准错误输出，详细内容仅保留在本机诊断文件', output_diagnostic: '运行依赖报告异常，请检查环境、端口与可用内存', launcher_started: '控制面板已启动', launcher_stopping: '控制面板正在关闭', launcher_failed: '控制面板启动失败' });
function entry(level, code, category) {
  return { id: randomUUID(), timestamp: new Date().toISOString(), level, source: 'launcher', category, code, summary: summaries[code] };
}

export class LauncherEventStore {
  constructor(path, { maxEntries = 1000, maxBytes = 512 * 1024 } = {}) {
    this.path = path; this.maxEntries = maxEntries; this.maxBytes = maxBytes; this.queue = Promise.resolve(); this.storageAvailable = true; this.memory = [];
    this.pendingWrites = 0; this.overflowed = false; this.initialized = false;
  }
  async record(level, code, category) {
    if (!levels.has(level) || !Object.hasOwn(summaries, code) || !categories.has(category)) return;
    const value = entry(level, code, category);
    this.memory.push(value);
    if (this.memory.length > this.maxEntries) this.memory.shift();
    if (this.pendingWrites >= this.maxEntries) { this.overflowed = true; this.storageAvailable = false; return; }
    this.pendingWrites++;
    this.queue = this.queue.then(async () => {
      await mkdir(dirname(this.path), { recursive: true, mode: 0o700 });
      if (!this.initialized) { await this.initialize(); this.initialized = true; }
      await appendFile(this.path, `${JSON.stringify(value)}\n`, { mode: 0o600 });
      await this.compact();
      this.storageAvailable = !this.overflowed;
    }).catch(() => { this.storageAvailable = false; }).finally(() => { this.pendingWrites--; });
    await this.queue;
  }
  async initialize() {
    const file = await open(this.path, 'a+', 0o600);
    try {
      const size = (await file.stat()).size;
      if (!size) return;
      const byte = Buffer.alloc(1);
      await file.read(byte, 0, 1, size - 1);
      if (byte[0] !== 10) await file.write('\n');
    } finally { await file.close(); }
  }
  async read() {
    const file = await open(this.path, 'r');
    try {
      const size = (await file.stat()).size;
      const offset = Math.max(0, size - this.maxBytes);
      const bytes = Buffer.alloc(Math.min(size, this.maxBytes));
      const { bytesRead } = await file.read(bytes, 0, bytes.length, offset);
      const text = bytes.toString('utf8', 0, bytesRead);
      return { text: offset ? text.slice(text.indexOf('\n') + 1) : text, oversized: offset > 0 };
    } finally { await file.close(); }
  }
  async compact() {
    const { text, oversized } = await this.read();
    if (!oversized && text.split('\n').filter(Boolean).length <= this.maxEntries) return;
    const values = text.split('\n').filter(Boolean).slice(-this.maxEntries).map(line => { try { return JSON.parse(line); } catch { return null; } }).filter(Boolean);
    while (values.length && Buffer.byteLength(values.map(value => `${JSON.stringify(value)}\n`).join('')) > this.maxBytes) values.shift();
    const temporary = `${this.path}.tmp`;
    await writeFile(temporary, values.map(value => `${JSON.stringify(value)}\n`).join(''), { mode: 0o600 });
    await rename(temporary, this.path);
  }
  async list({ level, source, category, query, limit = 100 } = {}) {
    await this.queue;
    let text = ''; try { ({ text } = await this.read()); } catch { this.storageAvailable = false; }
    const values = text.split('\n').filter(Boolean).map(line => { try { return JSON.parse(line); } catch { return null; } }).filter(value => value && levels.has(value.level) && value.source === 'launcher' && /^[a-f0-9-]{36}$/u.test(value.id) && /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z$/u.test(value.timestamp) && categories.has(value.category) && Object.hasOwn(summaries, value.code)).map(value => ({ id: value.id, timestamp: value.timestamp, level: value.level, source: 'launcher', category: value.category, code: value.code, summary: summaries[value.code] }));
    const combined = [...new Map([...values, ...this.memory].map(value => [value.id, value])).values()].sort((a, b) => a.timestamp.localeCompare(b.timestamp)).slice(-this.maxEntries);
    const keyword = String(query ?? '').trim().toLocaleLowerCase();
    const filtered = combined.filter(value => (!level || value.level === level) && (!source || value.source === source) && (!category || value.category === category) && (!keyword || `${value.source} ${value.category} ${value.code} ${value.summary}`.toLocaleLowerCase().includes(keyword)));
    const entries = filtered.slice(-Math.max(1, Math.min(1000, Number(limit) || 100))).reverse();
    return { entries, storage_available: this.storageAvailable && !this.overflowed, truncated: this.overflowed || filtered.length > entries.length };
  }
}
