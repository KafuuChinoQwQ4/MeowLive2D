#!/usr/bin/env node
// Build both native processes from the same checkout before packaging the app.
import { spawnSync } from 'node:child_process';
import { copyFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

const root = fileURLToPath(new URL('../', import.meta.url));
const target = 'x86_64-pc-windows-msvc';
const cross = process.platform !== 'win32';
const env = { ...process.env,
  CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS: `${process.env.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS ?? ''} -C target-feature=+crt-static`.trim(),
};
function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, env, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
run('cargo', [...(cross ? ['xwin'] : []), 'build', '--release', '--locked', '--target', target, '-p', 'meowlive-server']);
const binaries = join(root, 'target/windows-bundle');
await mkdir(binaries, { recursive: true });
await copyFile(join(root, `target/${target}/release/meowlive-server.exe`), join(binaries, `meowlive-server-${target}.exe`));
// Invoke the local CLI through Node so Windows needs no shell or global Tauri install.
run(process.execPath, [join(root, 'node_modules/@tauri-apps/cli/tauri.js'), 'build',
  '--config', join(root, 'apps/desktop/src-tauri/tauri.conf.json'),
  ...(cross ? ['--runner', 'cargo-xwin'] : []), '--target', target, '--ci', '--', '--locked']);
