import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
const alive = pid => { try { process.kill(pid, 0); return true; } catch { return false; } };

for (const signals of [['SIGINT'], ['SIGINT', 'SIGINT'], ['SIGTERM', 'SIGTERM'], ['SIGHUP']]) {
  test(`launcher waits for owned process cleanup after ${signals.join(' then ')}`, { timeout: 8000 }, async t => {
    const root = await mkdtemp(join(tmpdir(), 'meow-shutdown-'));
    const program = `
      import { Supervisor } from ${JSON.stringify(new URL('./supervisor.mjs', import.meta.url).href)};
      import { installShutdownHandlers } from ${JSON.stringify(new URL('./shutdown.mjs', import.meta.url).href)};
      const manager = new Supervisor({ setup: {}, stopMs: 500, definitions: [{
        id: 'server', url: 'http://127.0.0.1:1', cwd: ${JSON.stringify(root)}, env: process.env,
        command: process.execPath, args: ['-e', "process.on('SIGINT',()=>{}); console.log('ready'); setInterval(()=>{},1000)"],
        logPath: ${JSON.stringify(join(root, 'server.log'))},
      }] });
      installShutdownHandlers(async () => { process.send('stopping'); await manager.close(); process.disconnect(); });
      await manager.setEnabled('server', true);
      const child = manager.records.get('server').child;
      child.stdout.once('data', () => process.send({ pid: child.pid }));
    `;
    const launcher = spawn(process.execPath, ['--input-type=module', '-e', program], { stdio: ['ignore', 'pipe', 'pipe', 'ipc'] });
    let ownedPid;
    const exited = once(launcher, 'exit');
    t.after(async () => {
      launcher.kill('SIGKILL');
      if (ownedPid && alive(ownedPid)) process.kill(-ownedPid, 'SIGKILL');
      await exited;
      await rm(root, { recursive: true, force: true });
    });
    [{ pid: ownedPid }] = await once(launcher, 'message');
    const stopping = once(launcher, 'message');
    launcher.kill(signals[0]);
    if (signals.length > 1) {
      assert.equal((await stopping)[0], 'stopping');
      await wait(50);
      launcher.kill(signals[1]);
    }
    const [code, signal] = await exited;
    assert.equal(code, 0, `launcher exited before cleanup: ${signal}`);
    assert.equal(alive(ownedPid), false, 'owned service must be reaped before launcher exits');
  });
}
