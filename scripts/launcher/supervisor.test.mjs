import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createServer } from 'node:http';
import { Supervisor } from './supervisor.mjs';

async function fixture(t, { delay = 0, exit = false, startupMs = 3000 } = {}) {
  const root = await mkdtemp(join(tmpdir(), 'meow-launcher-'));
  const socket = createServer();
  await new Promise(resolve => socket.listen(0, '127.0.0.1', resolve));
  const port = socket.address().port;
  await new Promise(resolve => socket.close(resolve));
  const code = exit ? 'process.exit(7)' : `
    const http = require('node:http');
    setTimeout(() => http.createServer((req, res) => {
      res.setHeader('content-type','application/json');
      res.end(JSON.stringify({protocol_version:3,session_id:'test',bridge_connected:false,speeches:[]}));
    }).listen(${port}, '127.0.0.1'), ${delay});
  `;
  const definition = { id: 'server', url: `http://127.0.0.1:${port}`, command: process.execPath,
    args: ['-e', code], cwd: root, env: process.env, issue: null, logPath: join(root, 'server.log') };
  const manager = new Supervisor({ definitions: [definition], setup: {}, startupMs, stopMs: 300, pollMs: 30 });
  t.after(async () => { await manager.close(); await rm(root, { recursive: true, force: true }); });
  return { manager, port, definition };
}

async function until(manager, expected) {
  const end = Date.now() + 5000;
  while (Date.now() < end) {
    const status = (await manager.snapshot()).services[0];
    if (status.state === expected) return status;
    await new Promise(resolve => setTimeout(resolve, 25));
  }
  assert.fail(`did not reach ${expected}: ${JSON.stringify(await manager.snapshot())}`);
}

test('starts a real service once, waits for HTTP readiness and stops it', async t => {
  const { manager } = await fixture(t, { delay: 180 });
  assert.equal((await manager.snapshot()).services[0].state, 'stopped');
  await Promise.all([manager.setEnabled('server', true), manager.setEnabled('server', true)]);
  assert.equal((await manager.snapshot()).services[0].state, 'starting');
  assert.equal((await until(manager, 'running')).managed, true);
  await manager.setEnabled('server', false);
  assert.equal((await until(manager, 'stopped')).can_start, true);
});

test('can cancel a service while it is starting and restart it', async t => {
  const { manager } = await fixture(t, { delay: 1500 });
  await manager.setEnabled('server', true);
  await manager.setEnabled('server', false);
  await until(manager, 'stopped');
  await manager.setEnabled('server', true);
  await until(manager, 'running');
});

test('unexpected child exit is a visible failure, not running', async t => {
  const { manager } = await fixture(t, { exit: true });
  await manager.setEnabled('server', true);
  const state = await until(manager, 'failed');
  assert.match(state.message, /7/);
  assert.equal(state.can_start, true);
});

test('readiness timeout cleans up the owned child', async t => {
  const { manager } = await fixture(t, { delay: 5000, startupMs: 100 });
  await manager.setEnabled('server', true);
  const state = await until(manager, 'failed');
  assert.match(state.message, /超时/);
  assert.equal(state.managed, false);
});

test('observes an external service without taking ownership or stopping it', async t => {
  const { manager, port } = await fixture(t);
  const external = createServer((_req, res) => res.end(JSON.stringify({ protocol_version: 3, session_id: 'external', bridge_connected: false, speeches: [] })));
  await new Promise(resolve => external.listen(port, '127.0.0.1', resolve));
  t.after(() => new Promise(resolve => external.close(resolve)));
  const status = await until(manager, 'external');
  assert.equal(status.managed, false);
  assert.equal(status.can_stop, false);
  await assert.rejects(manager.setEnabled('server', false), /其他终端/);
  await manager.close();
  assert.equal((await fetch(`http://127.0.0.1:${port}`)).status, 200);
});

test('a different HTTP service on the port cannot be mistaken for our server', async t => {
  const { manager, port } = await fixture(t);
  const external = createServer((_req, res) => res.end('{}'));
  await new Promise(resolve => external.listen(port, '127.0.0.1', resolve));
  t.after(() => new Promise(resolve => external.close(resolve)));
  await assert.rejects(manager.setEnabled('server', true), /占用/);
  const status = await until(manager, 'failed');
  assert.equal(status.can_start, false);
});

test('closing the launcher reaps all owned service processes', async t => {
  const { manager, port } = await fixture(t);
  await manager.setEnabled('server', true);
  await until(manager, 'running');
  await manager.close();
  await assert.rejects(fetch(`http://127.0.0.1:${port}`, { signal: AbortSignal.timeout(500) }));
  await assert.rejects(manager.setEnabled('server', true), /退出/);
});


test('log locations are relative to the service checkout but writes keep their absolute target', async t => {
  const { manager, definition } = await fixture(t);
  const status = (await manager.snapshot()).services[0];
  assert.equal(status.log_path, './server.log');
  assert.equal(definition.logPath, join(definition.cwd, 'server.log'));
});
