import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, mkdir, writeFile, rm, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

async function engine(t, { ignore = false, orphanWorker = false } = {}) {
  const root = await mkdtemp(join(tmpdir(), 'meow-cleanup-'));
  const work = join(root, 'data/control-panel-inference/test/work');
  await mkdir(work, { recursive: true });
  const script = join(work, 'api_v2.py');
  await writeFile(script, `import signal,time,subprocess,sys\n${ignore ? 'signal.signal(signal.SIGINT,signal.SIG_IGN)\n' : ''}${orphanWorker ? 'subprocess.Popen([sys.executable,"-c","import signal,time; signal.signal(signal.SIGINT,signal.SIG_IGN); time.sleep(60)"],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)\ntime.sleep(.15)\n' : ''}print("ready",flush=True)\ntime.sleep(60)\n`);
  const child = spawn('python3', [script, '-a', '127.0.0.1', '-p', '9880', '-c', join(work, '../tts-infer.yaml')],
    { cwd: work, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
  const exited = once(child, 'exit');
  t.after(async () => {
    try { process.kill(-child.pid, 'SIGKILL'); } catch { /* already stopped */ }
    await exited;
    await rm(root, { recursive: true, force: true });
  });
  await once(child.stdout, 'data');
  return { root, child };
}

const running = async pid => {
  try { return !['Z', 'X'].includes((await readFile(`/proc/${pid}/stat`, 'utf8')).split(') ').at(-1).split(' ')[0]); }
  catch { return false; }
};

test('cleanup stops this checkout TTS, escalates an unresponsive process, and leaves another checkout running', async t => {
  const { stopProjectProcesses } = await import('./process-cleanup.mjs');
  const owned = await engine(t, { ignore: true });
  const external = await engine(t);
  const result = await stopProjectProcesses(owned.root, { graceMs: 100, forceMs: 1000, report: () => {} });
  assert.equal(await running(owned.child.pid), false);
  assert.equal(await running(external.child.pid), true);
  assert.deepEqual(result.remaining, []);
  assert.ok(result.forced.includes(owned.child.pid));
  assert.deepEqual((await stopProjectProcesses(owned.root, { report: () => {} })).remaining, []);
});

test('cleanup also reaps workers whose group leader exits on the first signal', async t => {
  const { stopProjectProcesses, scanProcesses } = await import('./process-cleanup.mjs');
  const owned = await engine(t, { orphanWorker: true });
  const members = (await scanProcesses()).filter(p => p.group === owned.child.pid);
  assert.equal(members.length, 2);
  const result = await stopProjectProcesses(owned.root, { graceMs: 150, forceMs: 1000, report: () => {} });
  assert.deepEqual(result.remaining, []);
  for (const member of members) assert.equal(await running(member.pid), false);
});

test('cleanup sends the Windows helper stop request and stops the matching panel before services', async t => {
  const { stopProjectProcesses } = await import('./process-cleanup.mjs');
  const owned = await engine(t);
  const script = join(owned.root, 'scripts/start-control-panel.mjs');
  await mkdir(join(owned.root, 'scripts'));
  await writeFile(script, 'console.log("ready"); setInterval(()=>{},1000);');
  const panel = spawn(process.execPath, [script], { cwd: owned.root, stdio: ['ignore', 'pipe', 'pipe'] });
  const exited = once(panel, 'exit');
  t.after(async () => { panel.kill('SIGKILL'); await exited; });
  await once(panel.stdout, 'data');
  const session = join(owned.root, 'data/windows-launcher/test');
  await mkdir(session, { recursive: true });
  await writeFile(join(session, 'lease'), 'test');
  const result = await stopProjectProcesses(owned.root, { graceMs: 500, report: () => {} });
  assert.equal(await running(panel.pid), false);
  assert.equal(await running(owned.child.pid), false);
  assert.equal(await readFile(join(session, 'stop'), 'utf8'), 'stop');
  assert.deepEqual(result.remaining, []);
});

test('cleanup stops this checkout Rust cache runner and its compile subprocesses', async t => {
  const { stopProjectProcesses } = await import('./process-cleanup.mjs');
  const root = await mkdtemp(join(tmpdir(), 'meow-cache-runner-'));
  await mkdir(join(root, 'scripts'));
  const script = join(root, 'scripts/rust_cache.py');
  await writeFile(script, 'import time\nprint("ready",flush=True)\ntime.sleep(60)\n');
  const child = spawn('python3', [script, 'run', '-p', 'meowlive-server'], {
    cwd: root, detached: true, stdio: ['ignore', 'pipe', 'pipe'],
  });
  const exited = once(child, 'exit');
  t.after(async () => {
    try { process.kill(-child.pid, 'SIGKILL'); } catch { /* stopped */ }
    await exited;
    await rm(root, { recursive: true, force: true });
  });
  await once(child.stdout, 'data');
  const result = await stopProjectProcesses(root, { graceMs: 300, report: () => {} });
  assert.equal(await running(child.pid), false);
  assert.deepEqual(result.remaining, []);
});


test('cleanup cancels the managed server wrapper during database preparation', async t => {
  const { stopProjectProcesses } = await import('./process-cleanup.mjs');
  const root = await mkdtemp(join(tmpdir(), 'meow-server-wrapper-'));
  await mkdir(join(root, 'scripts/launcher'), { recursive: true });
  const script = join(root, 'scripts/launcher/server.mjs');
  await writeFile(script, 'console.log("ready"); setInterval(()=>{},1000);');
  const child = spawn(process.execPath, [script], { cwd: root, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
  const exited = once(child, 'exit');
  t.after(async () => {
    try { process.kill(-child.pid, 'SIGKILL'); } catch { /* stopped */ }
    await exited;
    await rm(root, { recursive: true, force: true });
  });
  await once(child.stdout, 'data');
  const result = await stopProjectProcesses(root, { graceMs: 300, report: () => {} });
  assert.equal(await running(child.pid), false);
  assert.deepEqual(result.remaining, []);
});
