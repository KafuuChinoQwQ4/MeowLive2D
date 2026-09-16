import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { downloadModel, hasDownloadReceipt, modelManifest } from './model-download.mjs';
import { detectEnvironment } from './model-environment.mjs';

test('native Linux, WSL1 and WSL2 have distinct readiness', () => {
  assert.equal(detectEnvironment('6.10-arch', {}).kind, 'linux');
  assert.equal(detectEnvironment('6.6.114-microsoft-standard-WSL2', {}).ready, true);
  assert.equal(detectEnvironment('4.4.0-Microsoft', { WSL_DISTRO_NAME: 'Ubuntu' }).ready, false);
  assert.equal(detectEnvironment('unknown', { WSL_DISTRO_NAME: 'Ubuntu' }).ready, false);
});

async function fixture(t) {
  const directory = await mkdtemp(join(tmpdir(), 'meow-model-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const content = Buffer.from('a tiny test model with a verified checksum');
  const digest = createHash('sha256').update(content).digest('hex');
  const model = { id: 'test-model', sources: [{ repo: 'author/model', prefix: 'weights', include: ['model.bin'] }] };
  const data = { sha: 'a'.repeat(40), siblings: [{ rfilename: 'model.bin', size: content.length, lfs: { sha256: digest } },
    { rfilename: 'unrequested.bin', size: 999999 }] };
  const urls = [];
  const fetcher = async url => { urls.push(url); return url.includes('/api/') ? Response.json(data) : new Response(content); };
  return { directory, content, model, data, urls, fetcher };
}

test('downloads a pinned revision, verifies bytes and reuses valid completed files', async t => {
  const { directory, content, model, urls, fetcher } = await fixture(t);
  const progress = [];
  await downloadModel(model, directory, new AbortController().signal, (...args) => progress.push(args), { fetcher, reserveBytes: 0 });
  assert.deepEqual(await readFile(join(directory, 'weights/model.bin')), content);
  assert.equal(await hasDownloadReceipt(directory, model.id), true);
  assert.match(urls[1], /resolve\/a{40}\/model.bin$/);
  assert.equal(progress.at(-1)[0], content.length);
  urls.length = 0;
  await downloadModel(model, directory, new AbortController().signal, () => {}, { fetcher, reserveBytes: 0 });
  assert.equal(urls.length, 1);
});

test('corrupt or truncated model cannot acquire an installed receipt', async t => {
  const { directory, model, data } = await fixture(t);
  for (const content of ['short', 'x'.repeat(data.siblings[0].size)]) {
    await assert.rejects(downloadModel(model, directory, new AbortController().signal, () => {}, {
      fetcher: async url => url.includes('/api/') ? Response.json(data) : new Response(content), reserveBytes: 0,
    }), /校验失败/);
    assert.equal(await hasDownloadReceipt(directory, model.id), false);
  }
});

test('metadata rejects traversal, missing hashes and excessive disk demand', async t => {
  const { model, data } = await fixture(t);
  model.sources[0].include = [];
  for (const patch of [{ rfilename: '../outside' }, { lfs: undefined }, { size: 100 * 1024 ** 3 }]) {
    const altered = { ...data, siblings: [{ ...data.siblings[0], ...patch }] };
    await assert.rejects(modelManifest(model, new AbortController().signal, async () => Response.json(altered)));
  }
});

test('cancellation interrupts a body stream and leaves no completed model', async t => {
  const { directory, model, data } = await fixture(t);
  const controller = new AbortController();
  const fetcher = async url => url.includes('/api/') ? Response.json(data) : new Response(new ReadableStream({
    start(stream) { stream.enqueue(new Uint8Array([1])); },
    pull(stream) { controller.abort(); stream.enqueue(new Uint8Array([2])); },
  }));
  await assert.rejects(downloadModel(model, directory, controller.signal, () => {}, { fetcher, reserveBytes: 0 }));
  assert.equal(await hasDownloadReceipt(directory, model.id), false);
});

test('gated repositories give an actionable official-source error', async t => {
  const { model } = await fixture(t);
  await assert.rejects(modelManifest(model, new AbortController().signal, async () => new Response('', { status: 403 })), /登录或接受许可/);
});
