import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { ModelLibrary } from './model-library.mjs';
import { MODEL_CATALOG } from './model-catalog.mjs';

const turboId = 'faster-whisper-large-v3-turbo';
async function fixture(t, options = {}) {
  const root = await mkdtemp(join(tmpdir(), 'meow-asr-'));
  const configuration = { modelSettings: { root, engine: join(root, 'engine'), python: '/usr/bin/python3',
    environment: { ready: true } }, definitions: [{ id: 'tts', args: ['--data-dir', join(root, 'inference')] }] };
  let ttsChanges = 0;
  const supervisor = { configureTts: async update => { ttsChanges++; await update(); } };
  const library = new ModelLibrary(configuration, supervisor, { extraRoots: [], asrProbe: async () => true, ...options });
  t.after(async () => { await library.close(); await rm(root, { recursive: true, force: true }); });
  await library.initialize();
  const path = join(root, 'data/models', turboId);
  async function install() {
    await mkdir(path, { recursive: true });
    for (const file of ['model.bin', 'config.json', 'tokenizer.json', 'preprocessor_config.json']) {
      await writeFile(join(path, file), file === 'model.bin' ? Buffer.alloc(2048) : '{}');
    }
    await library.action('scan');
    return (await library.snapshot()).installed.find(item => item.model_id === turboId);
  }
  return { root, path, library, configuration, supervisor, install, ttsChanges: () => ttsChanges };
}

test('ASR catalog lists turbo and optional downloads without starting downloads', async t => {
  const { library } = await fixture(t, { downloader: async () => assert.fail('must be opt-in') });
  const snapshot = await library.snapshot();
  assert.equal(snapshot.catalog.find(model => model.id === turboId)?.compatibility, 'ready');
  assert.deepEqual(snapshot.catalog.filter(model => model.purpose === 'asr' && model.compatibility === 'download_only')
    .map(model => model.id).sort(), ['paraformer-zh', 'qwen3-asr-0.6b', 'qwen3-asr-1.7b', 'sensevoice-small']);
  assert.deepEqual(snapshot.downloads, []);
});

test('ASR selection persists independently of TTS and survives a fresh library', async t => {
  const f = await fixture(t);
  const item = await f.install();
  assert.ok(item?.ready, 'complete turbo weights should be selectable');
  assert.deepEqual((await f.library.snapshot()).installed.filter(model => model.purpose === 'asr').map(model => model.model_id), [turboId]);
  const before = f.ttsChanges();
  await f.library.action('select', item.id);
  assert.equal(f.ttsChanges(), before);
  const snapshot = await f.library.snapshot();
  assert.equal(snapshot.selected_id, null);
  assert.equal(snapshot.asr_selected_id, item.id);
  const selection = JSON.parse(await readFile(join(f.root, 'config/local/asr-model-selection.json'), 'utf8'));
  assert.equal(selection.path, f.path);
  assert.equal(selection.model_id, turboId);
  const again = new ModelLibrary(f.configuration, f.supervisor, { extraRoots: [], asrProbe: async () => true });
  t.after(() => again.close());
  await again.initialize();
  assert.equal((await again.snapshot()).asr_selected_id, item.id);
  assert.equal((await again.snapshot()).installed.filter(model => model.purpose === 'asr').length, 1);
  await rm(join(f.path, 'tokenizer.json'));
  await assert.rejects(f.library.action('select', item.id), /缺少/);
});

test('missing ASR dependency keeps downloaded weights unavailable with an actionable message', async t => {
  const f = await fixture(t, { asrProbe: async () => false });
  const item = await f.install();
  assert.ok(item);
  assert.equal(item.ready, false);
  assert.match(item.message, /faster-whisper/);
  await assert.rejects(f.library.action('select', item.id));
});

test('turbo download must contain loading files before it can complete', async t => {
  const { library } = await fixture(t, { downloader: async () => {} });
  await library.action('download', turboId);
  await library.jobs[0].completion;
  assert.equal((await library.snapshot()).downloads[0].state, 'failed');
  assert.match((await library.snapshot()).downloads[0].message, /不完整/);
});

test('a damaged saved selection can be repaired from the panel without falling back silently', async t => {
  const f = await fixture(t);
  const item = await f.install();
  await mkdir(join(f.root, 'config/local'), { recursive: true });
  await writeFile(join(f.root, 'config/local/asr-model-selection.json'), '{broken');
  await f.library.action('scan');
  assert.equal((await f.library.snapshot()).asr_selected_id, null);
  assert.match((await f.library.snapshot()).asr_error, /重新选择/);
  await f.library.action('select', item.id);
  assert.equal((await f.library.snapshot()).asr_error, null);
});
