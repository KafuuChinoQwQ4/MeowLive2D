import test from 'node:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { WindowsClientSupervisor, managedServices } from './windows-client.mjs';

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'meow-windows-'));
  let bridge = { ready: true, connected: false };
  const children = [], calls = [];
  const configuration = { modelSettings: { root, environment: { kind: 'wsl2' } }, definitions: [{ url: 'http://127.0.0.1:19600' }] };
  const manager = new WindowsClientSupervisor(configuration, {
    probe: async () => bridge, translate: async path => `Z:${path.replaceAll('/', '\\')}`,
    spawnHelper(command, args, options) {
      const child = new EventEmitter(); child.stdout = { resume() {} }; child.stderr = { resume() {} };
      child.kill = () => child.emit('close'); children.push(child); calls.push({ command, args, options }); return child;
    }, stopMs: 100, pollMs: 100_000,
  });
  manager.powershell = '/windows/powershell.exe';
  t.after(async () => { const closing = manager.close(); await new Promise(resolve => setTimeout(resolve, 10)); children.forEach(child => child.emit('close')); await closing; await rm(root, { recursive: true, force: true }); });
  return { manager, children, calls, setBridge: value => { bridge = value; }, root };
}

test('Windows start waits for main service and does not duplicate external clients', async t => {
  const f = await fixture(t);
  f.setBridge({ ready: false, connected: false });
  assert.equal((await f.manager.snapshot()).can_start, false);
  await assert.rejects(f.manager.setEnabled(true), /先打开主服务/);
  f.setBridge({ ready: true, connected: true });
  assert.equal((await f.manager.snapshot()).state, 'external');
  await f.manager.setEnabled(true);
  await assert.rejects(f.manager.setEnabled(false), /其他方式启动/);
  assert.equal(f.children.length, 0);
});

test('switch creates one fixed helper job and reports connected only after Windows and bridge readiness', async t => {
  const f = await fixture(t);
  await Promise.all([f.manager.setEnabled(true), f.manager.setEnabled(true)]);
  assert.equal(f.children.length, 1);
  assert.equal((await f.manager.snapshot()).state, 'starting');
  const session = f.manager.session;
  const job = JSON.parse(await readFile(join(session.path, 'job.json'), 'utf8'));
  assert.equal(job.schema_version, 1);
  assert.match(job.executable, /meowlive-client.exe$/);
  assert.ok(f.calls[0].args.includes('-File'));
  assert.equal(f.calls[0].options.shell, undefined);
  await writeFile(join(session.path, 'status.json'), JSON.stringify({ state: 'running', pid: 123 }));
  assert.equal((await f.manager.snapshot()).state, 'starting');
  f.setBridge({ ready: true, connected: true });
  assert.equal((await f.manager.snapshot()).state, 'running');
  await f.manager.setEnabled(false);
  assert.equal((await f.manager.snapshot()).state, 'stopping');
  assert.equal(await readFile(join(session.path, 'stop'), 'utf8'), 'stop');
  f.children[0].emit('close');
  f.setBridge({ ready: true, connected: false });
  assert.equal((await f.manager.snapshot()).state, 'stopped');
});

test('starting can be cancelled and unexpected helper exit can be retried', async t => {
  const f = await fixture(t);
  await f.manager.setEnabled(true);
  f.children[0].emit('close');
  assert.equal((await f.manager.snapshot()).state, 'failed');
  await f.manager.setEnabled(true);
  assert.equal(f.children.length, 2);
  const path = f.manager.session.path;
  await f.manager.setEnabled(false);
  assert.equal(await readFile(join(path, 'stop'), 'utf8'), 'stop');
  f.children[1].emit('close');
  assert.equal((await f.manager.snapshot()).state, 'stopped');
});

test('startup timeout asks Windows to stop and close waits for owned helper', async t => {
  const f = await fixture(t);
  await f.manager.setEnabled(true);
  f.manager.session.started = 0;
  await f.manager.refresh();
  assert.equal(f.manager.state, 'stopping');
  f.children[0].emit('close');
  assert.equal(f.manager.state, 'failed');
  await f.manager.setEnabled(true);
  const close = f.manager.close();
  await new Promise(resolve => setTimeout(resolve, 10));
  assert.equal(f.manager.state, 'stopping');
  f.children[1].emit('close');
  await close;
  assert.equal(f.manager.session, null);
});

test('main service stop waits for Windows disconnection while TTS control stays independent', async () => {
  const calls = [];
  const supervisor = { token: 'token', snapshot: async () => ({ services: [{ id: 'server' }, { id: 'tts' }] }), setEnabled: async (...args) => calls.push(args) };
  const windows = { snapshot: async () => ({ id: 'windows' }), setEnabled: async value => calls.push(['windows', value]), stopBeforeServer: async () => { calls.push(['windows-stop']); } };
  const manager = managedServices(supervisor, windows);
  assert.equal((await manager.snapshot()).services.length, 3);
  await manager.setEnabled('server', false);
  await manager.setEnabled('tts', false);
  await manager.setEnabled('windows', true);
  assert.deepEqual(calls, [['windows-stop'], ['server', false], ['tts', false], ['windows', true]]);
});


test('Windows log locations are project relative before and after startup', async t => {
  const f = await fixture(t);
  assert.equal((await f.manager.snapshot()).log_path, './data/windows-launcher');
  await f.manager.setEnabled(true);
  const snapshot = await f.manager.snapshot();
  assert.match(snapshot.log_path, /^\.\/data\/windows-launcher\/[a-f0-9-]+$/);
  assert.equal(snapshot.log_path.includes(f.root), false);
  assert.ok(f.manager.session.path.startsWith(f.root));
});
