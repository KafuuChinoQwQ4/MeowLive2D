import { access, mkdir, readdir, readFile, writeFile, rename, stat, realpath } from 'node:fs/promises';
import { constants } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { MODEL_CATALOG } from './model-catalog.mjs';
import { inspectGptModels, readSelection } from './model-environment.mjs';
import { downloadModel, hasDownloadReceipt } from './model-download.mjs';
import { displayPath, resolveConfiguredPath } from './paths.mjs';

const execute = promisify(execFile);
const installedId = (model, path) => createHash('sha256').update(`${model}:${path}`).digest('hex').slice(0, 24);
async function exists(path, mode = constants.R_OK) { try { await access(path, mode); return true; } catch { return false; } }
async function children(path) {
  try { return (await readdir(path, { withFileTypes: true })).filter(item => item.isDirectory()).slice(0, 100).map(item => join(path, item.name)); }
  catch { return []; }
}
async function markersPresent(path, markers) {
  if (!markers?.length) return false;
  for (const marker of markers.filter(marker => !marker.endsWith('.zip'))) {
    try {
      const info = await stat(join(path, marker));
      if (!info.isFile() || info.size < (/\.(?:pth|pt|bin|onnx|safetensors|ckpt|zip)$/.test(marker) ? 1024 : 1)) return false;
    } catch { return false; }
  }
  return true;
}

export class ModelLibrary {
  constructor(configuration, supervisor, { catalog = MODEL_CATALOG, downloader = downloadModel, extraRoots } = {}) {
    this.settings = configuration.modelSettings;
    this.supervisor = supervisor;
    this.originalTts = configuration.definitions.find(def => def.id === 'tts');
    this.catalog = catalog;
    this.downloader = downloader;
    this.storage = join(this.settings.root, 'data/models');
    this.home = this.settings.home ?? homedir();
    this.publicPath = value => displayPath(value, this.settings.root, this.home);
    this.roots = [...new Set([this.storage, this.settings.engine, ...(extraRoots ?? [
      join(this.settings.root, 'data/engines'), join(this.home, 'GPT-SoVITS'),
      this.settings.hfHome ? join(resolveConfiguredPath(this.settings.root, this.settings.hfHome, this.home), 'hub') : join(this.home, '.cache/huggingface/hub'),
    ])].filter(Boolean))];
    this.installed = []; this.jobs = []; this.selected = null; this.closed = false;
    this.queue = Promise.resolve();
  }

  async initialize() {
    this.selection = await readSelection(this.settings.root);
    await this.scan();
    const preferred = this.installed.find(item => item.id === this.selection?.id && item.path === this.selection?.path && item.ready)
      ?? this.installed.find(item => item.path === this.settings.engine && item.ready);
    try { await this.applySelection(preferred ?? null, false); }
    catch { /* An external TTS already listening is left untouched. */ }
  }

  async scan() {
    if (this.scanning) return this.scanning;
    this.scanning = this.performScan().finally(() => { this.scanning = null; });
    return this.scanning;
  }

  async performScan() {
    const { engine, python, environment } = this.settings;
    this.runtime = { engine_root: engine, python_path: python,
      ready: environment.ready && await exists(join(engine, 'api_v2.py')) && await exists(python, constants.X_OK),
      message: '' };
    this.runtime.message = this.runtime.ready ? '引擎代码与 Python 已找到，启动时会进一步检查依赖并加载模型。'
      : '尚未找到可用引擎和 Python。按 GPT-SoVITS 官方安装说明准备依赖，在 config/local/launcher.json 设置 ttsEngineRoot 与 ttsPython 后重启面板。';
    const candidates = new Set([engine, ...this.roots]);
    for (const root of this.roots) {
      for (const child of await children(root)) {
        candidates.add(child);
        // Common layouts: engine/pretrained_models/<model> and Hugging Face snapshots.
        for (const nested of await children(join(child, 'pretrained_models'))) candidates.add(nested);
        for (const snapshot of await children(join(child, 'snapshots'))) candidates.add(snapshot);
      }
    }
    const installed = [], seen = new Set();
    for (const path of [...candidates].slice(0, 600)) {
      let canonical; try { canonical = await realpath(path); } catch { continue; }
      if (seen.has(canonical)) continue; seen.add(canonical);
      for (const model of this.catalog) {
        if (model.compatibility === 'ready') {
          const inspection = await inspectGptModels(canonical);
          const partial = await exists(join(canonical, 'GPT_SoVITS/pretrained_models/gsv-v2final-pretrained'));
          if (!inspection.ready && !partial) continue;
          const ready = inspection.ready && this.runtime.ready;
          installed.push({ id: installedId(model.id, canonical), model_id: model.id, name: model.name, path: canonical,
            ready, selected: false, message: ready ? '文件检查通过，可选择并启动。' : inspection.ready ? this.runtime.message
              : `模型文件不完整，缺少 ${inspection.missing.length} 项。可在下载目录补齐完整模型。` });
        } else if (await hasDownloadReceipt(canonical, model.id) || (await this.manualIdentityMatches(canonical, model) && await markersPresent(canonical, model.markers))) {
          installed.push({ id: installedId(model.id, canonical), model_id: model.id, name: model.name, path: canonical,
            ready: false, selected: false, message: '已发现模型文件。本项目尚未适配该引擎，可查看官方说明使用。' });
        }
      }
    }
    this.installed = installed.slice(0, 256);
    this.markSelected();
  }

  markSelected() { this.installed.forEach(item => { item.selected = item.id === this.selected; }); }

  async manualIdentityMatches(path, model) {
    // A managed receipt is authoritative. Similar variants often share all filenames.
    try {
      const receipt = JSON.parse(await readFile(join(path, '.meowlive-model.json'), 'utf8'));
      return receipt.model_id === model.id;
    } catch { /* A manually installed directory may have no receipt. */ }
    const fingerprint = JSON.stringify([...(model.markers ?? [])].sort());
    if (!this.catalog.some(other => other.id !== model.id && JSON.stringify([...(other.markers ?? [])].sort()) === fingerprint)) return true;
    const normalized = path.toLowerCase();
    return normalized.includes(model.id.toLowerCase()) || model.sources.some(source => normalized.includes(source.repo.split('/')[1].toLowerCase()));
  }

  async applySelection(item, persist = true) {
    if (item && (!item.ready || !(await inspectGptModels(item.path)).ready)) throw new Error('所选模型缺少运行文件或尚未适配。');
    await this.supervisor.configureTts(async () => {
      if (persist) {
        const path = join(this.settings.root, 'config/local/model-selection.json');
        await mkdir(join(this.settings.root, 'config/local'), { recursive: true, mode: 0o700 });
        await writeFile(`${path}.tmp`, JSON.stringify(item ? { id: item.id, path: item.path } : {}), { mode: 0o600 });
        await rename(`${path}.tmp`, path);
      }
      const definition = { ...this.originalTts, args: [...this.originalTts.args] };
      if (item) {
        definition.args.push('--model-root', item.path);
        definition.args[definition.args.indexOf('--data-dir') + 1] = join(this.settings.root, 'data/control-panel-inference', item.id);
      } else definition.issue ??= '尚未发现完整的 GPT-SoVITS v2 模型，请先进入“环境与模型”下载或重新扫描。';
      this.selected = item?.id ?? null; this.markSelected();
      return definition;
    });
  }

  async snapshot() {
    return { schema_version: 1, environment: this.settings.environment,
      runtime: { ...this.runtime, engine_root: this.publicPath(this.runtime.engine_root), python_path: this.publicPath(this.runtime.python_path) },
      scan_roots: this.roots.map(this.publicPath),
      installed: this.installed.map(item => ({ ...item, path: this.publicPath(item.path) })), catalog: this.catalog.map(({ sources, markers, ...publicModel }) => publicModel),
      downloads: this.jobs.map(({ controller, completion, ...job }) => ({ ...job, path: this.publicPath(job.path) })), selected_id: this.selected };
  }

  async action(action, id) {
    const result = this.queue.then(async () => {
      if (this.closed) throw new Error('控制面板正在退出。');
      if (action === 'scan') { await this.scan(); return; }
      if (action === 'select') {
        const item = this.installed.find(entry => entry.id === id);
        if (!item) throw new Error('未找到所选模型，请重新扫描。');
        await this.applySelection(item); return;
      }
      if (action === 'cancel') {
        const job = this.jobs.find(entry => entry.id === id);
        if (!job) throw new Error('未找到下载任务。');
        job.controller.abort(); await job.completion; return;
      }
      if (action !== 'download') throw new Error('未知模型操作。');
      const model = this.catalog.find(entry => entry.id === id);
      if (!model) throw new Error('模型不在官方来源目录中。');
      if (!this.settings.environment.ready) throw new Error(this.settings.environment.message);
      if (this.jobs.some(job => job.state === 'downloading')) throw new Error('已有下载正在进行，请等待完成或取消后再下载。');
      const job = { id: randomUUID(), model_id: id, state: 'downloading', message: '正在获取官方文件清单…',
        downloaded_bytes: 0, total_bytes: 0, path: join(this.storage, id), controller: new AbortController() };
      this.jobs = [...this.jobs.filter(entry => entry.model_id !== id).slice(-19), job];
      job.completion = this.runDownload(model, job);
    });
    this.queue = result.catch(() => {});
    await result;
  }

  async runDownload(model, job) {
    try {
      await this.downloader(model, job.path, job.controller.signal, (done, total, message) => {
        job.downloaded_bytes = done; job.total_bytes = total; job.message = message;
      });
      job.controller.signal.throwIfAborted();
      if (model.id.startsWith('gpt-sovits-')) {
        job.message = '正在解压中文读音模型…';
        await execute('python3', [join(this.settings.root, 'scripts/extract-model-archive.py'), join(job.path, 'G2PWModel.zip'),
          join(job.path, 'GPT_SoVITS/text')], { signal: job.controller.signal, timeout: 120_000, maxBuffer: 4096 });
      }
      await this.scan();
      if (model.compatibility === 'ready' && !(await inspectGptModels(job.path)).ready) throw new Error('下载已结束，但运行文件不完整，请查看官方安装说明。');
      job.controller.signal.throwIfAborted();
      job.state = 'completed'; job.message = model.compatibility === 'ready'
        ? '下载与校验完成，请在“本地模型”选择；运行环境缺失时请先安装引擎。' : '模型文件已下载，尚需按官方说明安装引擎并适配本项目接口。';
    } catch (error) {
      job.state = job.controller.signal.aborted ? 'cancelled' : 'failed';
      job.message = job.state === 'cancelled' ? '已取消。再次下载时会复用已完成且校验通过的文件。'
        : error.code === 'ENOSPC' ? '磁盘空间不足，请清理后重试。'
          : error.path || ['EACCES', 'EPERM', 'ENOENT', 'EIO', 'EROFS'].includes(error.code)
            ? '无法读写模型文件，请检查下方下载目录及其权限后重试。' : error.message?.startsWith('Command failed')
          ? '中文模型解压失败，请检查 Python 和磁盘空间后重试。' : /fetch failed|timeout|aborted/i.test(error.message)
            ? '无法连接官方模型源或连接超时，请检查 WSL 网络后重试，也可打开官方页面手动下载。' : error.message;
    }
  }

  async close() {
    this.closed = true;
    this.jobs.forEach(job => job.controller.abort());
    await Promise.allSettled(this.jobs.map(job => job.completion));
  }
}
