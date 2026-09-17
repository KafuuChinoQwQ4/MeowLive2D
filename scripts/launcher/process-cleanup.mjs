import { readFile, readlink, readdir, stat, realpath, writeFile } from 'node:fs/promises';
import { constants } from 'node:fs';
import { basename, join, resolve, sep } from 'node:path';

const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
const inside = (path, parent) => path.startsWith(`${parent}${sep}`);

async function identity(pid) {
  try {
    const fields = (await readFile(`/proc/${pid}/stat`, 'utf8')).split(') ').at(-1).trim().split(/\s+/);
    if (['Z', 'X'].includes(fields[0])) return null;
    return { pid, group: Number(fields[2]), session: Number(fields[3]), start: fields[19] };
  } catch (error) {
    if (['ENOENT', 'ESRCH'].includes(error.code)) return null;
    throw error;
  }
}

export async function scanProcesses() {
  const result = [];
  for (const entry of await readdir('/proc')) {
    if (!/^\d+$/.test(entry) || Number(entry) === process.pid) continue;
    try {
      if ((await stat(`/proc/${entry}`)).uid !== process.getuid()) continue;
      const before = await identity(Number(entry));
      if (!before) continue;
      const [cwd, executable, cmdline] = await Promise.all([
        readlink(`/proc/${entry}/cwd`), readlink(`/proc/${entry}/exe`), readFile(`/proc/${entry}/cmdline`, 'utf8'),
      ]);
      if ((await identity(before.pid))?.start !== before.start) continue;
      result.push({ ...before, cwd, executable: executable.replace(/ \(deleted\)$/, ''), args: cmdline.split('\0').filter(Boolean) });
    } catch (error) {
      if (!['ENOENT', 'ESRCH', 'EACCES', 'EPERM'].includes(error.code)) throw error;
    }
  }
  return result;
}

function serviceKind(record, root) {
  const { cwd, executable, args } = record;
  const script = args[1] && resolve(cwd, args[1]);
  if (cwd === root && /^node(?:js)?$/.test(basename(executable)) && script === join(root, 'scripts/start-control-panel.mjs')) return '控制面板';
  if (cwd === root && ['debug', 'release'].some(profile => executable === join(root, `target/${profile}/meowlive-server`))) return '主服务';
  if (cwd === root && basename(executable) === 'cargo' && args[1] === 'run'
      && args.some((arg, index) => arg === '-p' && args[index + 1] === 'meowlive-server')) return '主服务编译进程';
  if (!/^python(?:\d+(?:\.\d+)*)?$/.test(basename(executable))) return null;
  const inferenceRoot = join(root, 'data/control-panel-inference');
  if (cwd === root && script === join(root, 'scripts/start-managed-inference.py')) return 'TTS 启动进程';
  const api = args[1] === '-s' ? args[2] : args[1];
  if (api && inside(cwd, inferenceRoot) && basename(cwd) === 'work' && resolve(cwd, api) === join(cwd, 'api_v2.py')) return 'TTS';
  return null;
}

async function matching(records) {
  return (await Promise.all(records.map(async record => (await identity(record.pid))?.start === record.start ? record : null))).filter(Boolean);
}

async function requestWindowsStop(root) {
  const parent = join(root, 'data/windows-launcher');
  let sessions;
  try {
    if (await realpath(parent) !== parent) throw new Error('Windows 会话目录不能是符号链接。');
    sessions = await readdir(parent, { withFileTypes: true });
  } catch (error) { if (error.code === 'ENOENT') return; throw error; }
  for (const session of sessions.filter(entry => entry.isDirectory())) {
    await writeFile(join(parent, session.name, 'stop'), 'stop',
      { mode: 0o600, flag: constants.O_WRONLY | constants.O_CREAT | constants.O_TRUNC | constants.O_NOFOLLOW });
  }
}

export async function stopProjectProcesses(directory, { graceMs = 30_000, forceMs = 3000, report = console.log } = {}) {
  const root = await realpath(directory);
  const forced = [], remaining = [];
  await requestWindowsStop(root);
  // Close the panel first so its normal cleanup runs and it cannot start new services.
  for (const panelPhase of [true, false]) {
    const processes = await scanProcesses();
    const targets = processes.filter(record => {
      record.kind = serviceKind(record, root);
      return record.kind && (record.kind === '控制面板') === panelPhase;
    });
    await Promise.all(targets.map(async target => {
      const group = target.group === target.pid && target.session === target.pid;
      const members = group ? processes.filter(record => record.group === target.pid) : [target];
      async function signal(name) {
        if (!(await matching(members)).length) return;
        try { process.kill(group ? -target.pid : target.pid, name); }
        catch (error) { if (error.code !== 'ESRCH') throw error; }
      }
      async function waitUntilStopped(timeout) {
        const deadline = Date.now() + timeout;
        while ((await matching(members)).length && Date.now() < deadline) await wait(50);
        return matching(members);
      }
      report(`正在停止${target.kind}（PID ${target.pid}）…`);
      await signal('SIGINT');
      if ((await waitUntilStopped(graceMs)).length) {
        report(`${target.kind}未在等待时间内退出，强制回收其进程组。`);
        forced.push(target.pid);
        await signal('SIGKILL');
        remaining.push(...(await waitUntilStopped(forceMs)).map(record => record.pid));
      }
    }));
  }
  // Catch a matching service created while the first snapshot was being collected.
  for (const record of await scanProcesses()) if (serviceKind(record, root)) remaining.push(record.pid);
  return { forced, remaining: [...new Set(remaining)] };
}
