import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, open, writeFile, readFile, rm } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { createServer } from 'node:http';
import { ModelLibrary } from './model-library.mjs';
import { Supervisor } from './supervisor.mjs';
import { detectEnvironment, gptRequiredFiles } from './model-environment.mjs';

const catalog = [{ id: 'gpt-sovits-v2', name: 'GPT-SoVITS v2', compatibility: 'ready', sources: [] }];
async function weights(path) {
  for (const [relative, size] of gptRequiredFiles) {
    const file = join(path, relative); await mkdir(dirname(file), { recursive: true });
    const handle = await open(file, 'w'); await handle.truncate(size); await handle.close();
  }
}
async function fixture(t, installed = true, options = {}) {
  const root = await mkdtemp(join(tmpdir(), 'meow-library-'));
  const engine = join(root, 'engine');
  await mkdir(engine); await writeFile(join(engine, 'api_v2.py'), '# engine');
  if (installed) await weights(engine);
  const server = createServer((req, res) => { res.writeHead(200, { 'Content-Type': 'application/json' }); res.end(JSON.stringify({ paths: { '/tts': { post: {} } } })); });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const port = server.address().port;
  await new Promise(resolve => server.close(resolve));
  const configuration = { modelSettings: { root, engine, python: process.execPath, environment: detectEnvironment('linux', {}) },
    setup: {}, definitions: [{ id: 'tts', issue: null, args: ['--data-dir', join(root, 'data/inference')], url: `http://127.0.0.1:${port}` }] };
  const supervisor = new Supervisor(configuration);
  const library = new ModelLibrary(configuration, supervisor, { catalog, extraRoots: [], ...options });
  t.after(async () => { await library.close(); await supervisor.close(); if (server.listening) await new Promise(resolve => server.close(resolve)); await rm(root, { recursive: true, force: true }); });
  await library.initialize();
  return { root, engine, library, configuration, supervisor, server, port };
}

test('existing full GPT models are selected while empty installations block TTS startup', async t => {
  const existing = await fixture(t);
  assert.equal((await existing.library.snapshot()).installed[0].selected, true);
  const empty = await fixture(t, false);
  assert.deepEqual((await empty.library.snapshot()).installed, []);
  assert.equal((await empty.supervisor.snapshot()).services[0].can_start, false);
});

test('local model selection persists a discovered path and changes the inference arguments', async t => {
  const { root, library, supervisor } = await fixture(t);
  const other = join(root, 'data/models/gpt-sovits-v2'); await weights(other);
  await library.action('scan');
  const item = (await library.snapshot()).installed.find(item => item.path === './data/models/gpt-sovits-v2');
  assert.ok(item?.ready);
  await library.action('select', item.id);
  assert.equal((await library.snapshot()).selected_id, item.id);
  assert.equal(JSON.parse(await readFile(join(root, 'config/local/model-selection.json'), 'utf8')).path, other);
  assert.equal(supervisor.records.get('tts').def.args.at(-1), other);
  await rm(join(other, gptRequiredFiles[0][0]));
  await assert.rejects(library.action('select', item.id), /缺少运行文件/);
});

test('switching cannot replace an externally running TTS service', async t => {
  const { library, server, port } = await fixture(t);
  await new Promise(resolve => server.listen(port, '127.0.0.1', resolve));
  const item = (await library.snapshot()).installed[0];
  await assert.rejects(library.action('select', item.id), /先关闭 TTS/);
  assert.equal(server.listening, true);
});

test('downloads serialize and cancellation reaches a terminal visible state', async t => {
  const { library } = await fixture(t, true, { downloader: (_model, _path, signal) => new Promise((_resolve, reject) => {
    signal.addEventListener('abort', () => reject(signal.reason), { once: true });
  }) });
  await library.action('download', 'gpt-sovits-v2');
  await assert.rejects(library.action('download', 'gpt-sovits-v2'), /已有下载/);
  const job = (await library.snapshot()).downloads[0];
  await library.action('cancel', job.id);
  assert.equal((await library.snapshot()).downloads[0].state, 'cancelled');
  await assert.rejects(library.action('download', 'arbitrary-repo'), /官方来源目录/);
});

test('variants with identical filenames are not both inferred from one model directory', async t => {
  const models = ['small', 'large'].map(id => ({ id, name: id, compatibility: 'download_only',
    markers: ['config.json'], sources: [{ repo: `author/${id}` }] }));
  const { root, library } = await fixture(t, false, { catalog: models });
  const path = join(root, 'data/models/small');
  await mkdir(path, { recursive: true }); await writeFile(join(path, 'config.json'), '{}');
  await library.action('scan');
  assert.deepEqual((await library.snapshot()).installed.map(item => item.model_id), ['small']);
});

test('a cancellation during final scanning is not reported as completed', async t => {
  const { library } = await fixture(t, false, { catalog: [{ id: 'test', compatibility: 'download_only' }], downloader: async () => {} });
  library.scan = async () => { library.jobs[0].controller.abort(); };
  await library.action('download', 'test');
  await library.jobs[0].completion;
  assert.equal((await library.snapshot()).downloads[0].state, 'cancelled');
});


test('model snapshots show relative paths while inference keeps its resolved filesystem path', async t => {
  const { root, engine, library, supervisor } = await fixture(t);
  const snapshot = await library.snapshot();
  assert.equal(snapshot.runtime.engine_root, './engine');
  assert.equal(snapshot.installed[0].path, './engine');
  assert.deepEqual(snapshot.scan_roots, ['./data/models', './engine']);
  assert.equal(supervisor.records.get('tts').def.args.at(-1), engine);
  assert.equal(JSON.stringify(snapshot).includes(root), false);
});

test('filesystem failures keep the download location portable without echoing native absolute paths', async t => {
  const { root, library } = await fixture(t, false, {
    downloader: async (_model, destination) => { await readFile(join(destination, 'missing.bin')); },
  });
  await library.action('download', 'gpt-sovits-v2');
  await library.jobs[0].completion;
  const job = (await library.snapshot()).downloads[0];
  assert.equal(job.state, 'failed');
  assert.equal(job.path, './data/models/gpt-sovits-v2');
  assert.equal(job.message.includes(root), false);
  assert.match(job.message, /文件/);
});
