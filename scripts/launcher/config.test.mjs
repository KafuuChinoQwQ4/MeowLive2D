import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { loadConfiguration } from './config.mjs';

async function fixture(t, overrides = {}) {
  const root = await mkdtemp(join(tmpdir(), 'meow-config-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(join(root, 'config/local'), { recursive: true });
  await mkdir(join(root, 'engine'));
  await writeFile(join(root, 'engine/api_v2.py'), '');
  await writeFile(join(root, 'key.txt'), 'sk-test-secret-123');
  await writeFile(join(root, 'config/server.local.toml'), '[server]\nlisten_address="127.0.0.1:19600"\n[speech]\nbase_url="http://127.0.0.1:9880"\n[llm]\nbase_url="https://api.example.com"\nmodel="test"\napi_key_env="TEST_LLM_KEY"\n');
  await writeFile(join(root, 'config/local/launcher.json'), JSON.stringify({ serverConfig: 'config/server.local.toml', ttsPython: '/usr/bin/python3', ttsEngineRoot: 'engine', ttsDevice: 'cpu', llmKeyFile: 'key.txt', ...overrides }));
  return root;
}

test('loads the existing key into only the server environment, never public setup', async t => {
  const root = await fixture(t);
  const config = await loadConfiguration(root, { env: { PATH: process.env.PATH } });
  assert.equal(config.definitions.find(s => s.id === 'server').env.TEST_LLM_KEY, 'sk-test-secret-123');
  assert.equal(config.definitions.find(s => s.id === 'tts').env.TEST_LLM_KEY, undefined);
  assert.equal(config.setup.llm_configured, true);
  assert.equal(JSON.stringify(config.setup).includes('sk-test'), false);
});

test('an existing key environment variable takes precedence over the local key file', async t => {
  const root = await fixture(t);
  const config = await loadConfiguration(root, { env: { PATH: process.env.PATH, TEST_LLM_KEY: 'env-secret' } });
  assert.equal(config.definitions[0].env.TEST_LLM_KEY, 'env-secret');
  assert.equal(config.definitions[1].env.TEST_LLM_KEY, undefined);
});

test('missing TTS environment remains actionable without preventing the panel from opening', async t => {
  const root = await fixture(t, { ttsPython: '/missing/python' });
  const config = await loadConfiguration(root);
  assert.match(config.definitions.find(s => s.id === 'tts').issue, /Python/);
  assert.equal(config.definitions[0].issue, null);
});

test('a malformed launcher config disables starts with a safe diagnostic', async t => {
  const root = await fixture(t);
  await writeFile(join(root, 'config/local/launcher.json'), '{broken');
  const config = await loadConfiguration(root);
  assert.ok(config.definitions.every(service => /JSON/.test(service.issue)));
});

test('remote speech URLs cannot become local process control targets', async t => {
  const root = await fixture(t);
  await writeFile(join(root, 'config/server.local.toml'), '[server]\nlisten_address="127.0.0.1:19600"\n[speech]\nbase_url="http://example.com:9880"\n');
  const config = await loadConfiguration(root);
  assert.match(config.definitions.find(s => s.id === 'tts').issue, /127.0.0.1/);
});

test('an enabled cloud model with a missing key gives a preflight error', async t => {
  const root = await fixture(t, { llmKeyFile: 'missing-key.txt' });
  const config = await loadConfiguration(root, { env: { PATH: process.env.PATH } });
  assert.equal(config.setup.llm_configured, false);
  assert.match(config.definitions[0].issue, /密钥缺失/);
});


test('public setup is portable while service execution still uses the real checkout', async t => {
  const root = await fixture(t);
  const config = await loadConfiguration(root);
  assert.equal(config.setup.configuration_path, './config/local/launcher.json');
  assert.equal(config.setup.server_config, './config/server.local.toml');
  assert.equal(config.setup.windows_client_path, './target/windows-client');
  assert.equal(config.definitions[0].args.at(-1), join(root, 'config/server.local.toml'));
  assert.equal(config.definitions[0].cwd, root);
});

test('empty launcher settings resolve the engine and interpreter inside the checkout', async t => {
  const root = await fixture(t);
  await writeFile(join(root, 'config/local/launcher.json'), '{}');
  const config = await loadConfiguration(root);
  assert.equal(config.modelSettings.engine, join(root, 'data/engines/GPT-SoVITS'));
  assert.equal(config.modelSettings.python, join(root, 'data/environments/gpt-sovits/bin/python'));
});

test('home-relative settings resolve against the launch user home, including public setup', async t => {
  const root = await fixture(t, { ttsEngineRoot: '~/voice/engine', ttsPython: '~/voice/bin/python' });
  const config = await loadConfiguration(root, { env: { PATH: process.env.PATH, HOME: '/home/alice' } });
  assert.equal(config.modelSettings.engine, '/home/alice/voice/engine');
  assert.equal(config.modelSettings.python, '/home/alice/voice/bin/python');
});
