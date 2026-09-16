import { mkdir, open, rename, stat, statfs, readFile, writeFile } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join } from 'node:path';
import { fetch as httpFetch, EnvHttpProxyAgent } from 'undici';

const dispatcher = new EnvHttpProxyAgent({ headersTimeout: 60_000, bodyTimeout: 300_000 });
const networkFetch = (url, options) => httpFetch(url, { ...options, dispatcher });
export const closeModelConnections = () => dispatcher.destroy();

const MAX_BYTES = 60 * 1024 ** 3;
export function safeRelative(path) {
  return typeof path === 'string' && path.length < 1024 && !path.includes('\\') && !path.includes('\0')
    && path.split('/').every(part => part && part !== '.' && part !== '..');
}
function matches(path, patterns) {
  return !patterns?.length || patterns.some(pattern => pattern.endsWith('/')
    ? path.startsWith(pattern) : path === pattern);
}
async function request(url, signal, fetcher) {
  const headers = new AbortController();
  const timer = setTimeout(() => headers.abort(), 60_000);
  let response;
  try { response = await fetcher(url, { signal: AbortSignal.any([signal, headers.signal]), redirect: 'follow' }); }
  finally { clearTimeout(timer); }
  if (!response.ok) {
    await response.body?.cancel();
    throw new Error(response.status === 401 || response.status === 403
      ? '官方仓库需要登录或接受许可，请打开官方模型页面完成操作后手动下载。'
      : `模型源返回 HTTP ${response.status}，请稍后重试。`);
  }
  return response;
}
export async function modelManifest(model, signal, fetcher = networkFetch) {
  const files = [];
  for (const source of model.sources) {
    if (!/^[\w.-]+\/[\w.-]+$/.test(source.repo)) throw new Error('模型目录中的仓库标识无效。');
    const response = await request(`https://huggingface.co/api/models/${source.repo}/revision/main?blobs=true`, signal, fetcher);
    const chunks = []; let size = 0;
    for await (const chunk of response.body) {
      size += chunk.length; if (size > 4 * 1024 ** 2) throw new Error('官方文件清单过大。'); chunks.push(chunk);
    }
    const data = JSON.parse(Buffer.concat(chunks).toString());
    if (!/^[a-f0-9]{40}$/.test(data.sha) || !Array.isArray(data.siblings)) throw new Error('官方模型文件清单无效。');
    let matched = 0;
    for (const item of data.siblings) {
      if (!matches(item.rfilename, source.include)) continue;
      const relative = source.prefix ? `${source.prefix}/${item.rfilename}` : item.rfilename;
      const bytes = item.size ?? item.lfs?.size;
      const digest = item.lfs?.sha256 ?? item.blobId;
      if (!safeRelative(relative) || !Number.isSafeInteger(bytes) || bytes < 0 || !/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/.test(digest ?? '')) throw new Error('模型文件路径或校验信息无效。');
      files.push({ path: relative, bytes, digest, url: `https://huggingface.co/${source.repo}/resolve/${data.sha}/${item.rfilename.split('/').map(encodeURIComponent).join('/')}` });
      matched++;
    }
    if (!matched) throw new Error('官方仓库未找到所需权重，请查看模型说明。');
  }
  if (files.length > 4000 || new Set(files.map(file => file.path)).size !== files.length
      || files.reduce((sum, file) => sum + file.bytes, 0) > MAX_BYTES) throw new Error('模型文件数量或大小超过本地下载上限（60 GiB），请到官方页面手动下载。');
  return files;
}

function hashFor(file) {
  const hash = createHash(file.digest.length === 64 ? 'sha256' : 'sha1');
  if (file.digest.length === 40) hash.update(`blob ${file.bytes}\0`);
  return hash;
}
async function validExisting(path, file, signal) {
  try {
    if ((await stat(path)).size !== file.bytes) return false;
    const hash = hashFor(file);
    for await (const chunk of createReadStream(path, { signal })) hash.update(chunk);
    return hash.digest('hex') === file.digest;
  } catch { signal.throwIfAborted(); return false; }
}

export async function downloadModel(model, destination, signal, progress, { fetcher = networkFetch,
  reserveBytes = model.id.startsWith('gpt-sovits-') ? 2 * 1024 ** 3 : 256 * 1024 ** 2 } = {}) {
  const manifest = await modelManifest(model, signal, fetcher);
  const total = manifest.reduce((sum, file) => sum + file.bytes, 0);
  await mkdir(destination, { recursive: true, mode: 0o700 });
  progress(0, total, '正在校验已下载文件，可复用的文件无需重新下载。');
  const completed = new Set();
  let remaining = 0;
  for (const file of manifest) {
    signal.throwIfAborted();
    if (await validExisting(join(destination, file.path), file, signal)) completed.add(file.path);
    else remaining += file.bytes;
  }
  const disk = await statfs(destination);
  if (disk.bavail * disk.bsize < remaining + reserveBytes) throw new Error('可用磁盘空间不足，请清理模型保存目录所在磁盘后重试。');
  let downloaded = 0;
  progress(downloaded, total, '已获取文件清单，正在下载并校验。');
  for (const file of manifest) {
    signal.throwIfAborted();
    const target = join(destination, file.path);
    if (completed.has(file.path)) { downloaded += file.bytes; progress(downloaded, total, `已复用：${file.path}`); continue; }
    await mkdir(dirname(target), { recursive: true, mode: 0o700 });
    const response = await request(file.url, signal, fetcher);
    const handle = await open(`${target}.part`, 'w', 0o600);
    let received = 0;
    const hash = hashFor(file);
    try {
      for await (const chunk of response.body) {
        signal.throwIfAborted(); received += chunk.length;
        if (received > file.bytes) throw new Error('模型文件超过清单声明的大小，已停止下载。');
        hash.update(chunk); await handle.writeFile(chunk);
        progress(downloaded + received, total, `正在下载：${file.path}`);
      }
    } finally { await handle.close(); }
    if (received !== file.bytes || hash.digest('hex') !== file.digest) throw new Error('模型文件校验失败，请重试；已完成的文件会自动复用。');
    await rename(`${target}.part`, target);
    downloaded += file.bytes;
  }
  await writeFile(join(destination, '.meowlive-model.json'), JSON.stringify({ model_id: model.id, files: manifest.map(({ path, bytes, digest }) => ({ path, bytes, digest })) }), { mode: 0o600 });
  return manifest;
}

export async function hasDownloadReceipt(path, modelId) {
  try {
    const info = await stat(join(path, '.meowlive-model.json')); if (info.size > 2 * 1024 ** 2) return false;
    const receipt = JSON.parse(await readFile(join(path, '.meowlive-model.json'), 'utf8'));
    if (receipt.model_id !== modelId || !Array.isArray(receipt.files) || !receipt.files.length) return false;
    for (const file of receipt.files) {
      if (!safeRelative(file.path) || (await stat(join(path, file.path))).size !== file.bytes) return false;
    }
    return true;
  } catch { return false; }
}
