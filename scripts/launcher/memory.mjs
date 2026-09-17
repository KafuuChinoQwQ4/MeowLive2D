import { readFile } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { detectEnvironment } from './model-environment.mjs';

const execute = promisify(execFile);

// WSL's MemAvailable is not the Windows host's physical or commit headroom.
export async function readSystemMemory() {
  const info = await readFile('/proc/meminfo', 'utf8');
  const available = /^MemAvailable:\s+(\d+) kB$/m.exec(info);
  if (!available) throw new Error('memory unavailable');
  const result = [{ name: 'Linux / WSL', availableMiB: Number(available[1]) / 1024 }];
  if (detectEnvironment().kind === 'wsl2') {
    const script = '$m = Get-CimInstance Win32_OperatingSystem; [pscustomobject]@{availableMiB=$m.FreePhysicalMemory/1024;commitAvailableMiB=$m.FreeVirtualMemory/1024} | ConvertTo-Json -Compress';
    const { stdout } = await execute('/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe',
      ['-NoLogo', '-NoProfile', '-NonInteractive', '-Command', script], { timeout: 4000, maxBuffer: 4096 });
    const host = JSON.parse(stdout.trim());
    for (const key of ['availableMiB', 'commitAvailableMiB']) {
      if (!Number.isFinite(host[key]) || host[key] < 0) throw new Error('invalid host memory');
    }
    result.push({ name: 'Windows', ...host });
  }
  return result;
}

export function memoryPressure(samples, starting) {
  // Allow startup with 1 GiB free while retaining the runtime low-memory guard.
  const minimum = starting ? 1024 : 768;
  const low = samples.find(sample => sample.availableMiB < minimum
    || (sample.commitAvailableMiB !== undefined && sample.commitAvailableMiB < minimum));
  if (!low) return null;
  return `${low.name} 内存不足（可用 ${Math.floor(low.availableMiB)} MiB${low.commitAvailableMiB === undefined ? '' : `，可提交 ${Math.floor(low.commitAvailableMiB)} MiB`}），${starting ? '暂不启动 TTS' : '已停止 TTS 释放内存'}。请关闭本地大模型或其他高占用程序后手动重试。`;
}

export function createMemoryReader() {
  let expires = 0, pending;
  return () => {
    if (!pending || Date.now() >= expires) {
      expires = Date.now() + 5000;
      pending = readSystemMemory().catch(error => { expires = 0; throw error; });
    }
    return pending;
  };
}
