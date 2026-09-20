// Run preparation inside the supervised process so the panel can keep polling
// and cancel startup while Docker is getting PostgreSQL ready.
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { join, resolve } from 'node:path';
import { readServerConfig } from './config.mjs';
import { prepareViewerDatabase } from './database.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const args = process.argv.slice(2);
if (args.length > 2 || (args[0] === '--config' ? args.length !== 2 : args.length > 1)
    || (args[0]?.startsWith('-') && args[0] !== '--config')) {
  console.error('用法：npm run start:server -- --config config/server.local.toml');
  process.exit(1);
}
const configPath = resolve((args[0] === '--config' ? args[1] : args[0]) ?? join(root, 'config/server.example.toml'));
let metadata;
try { metadata = await readServerConfig(configPath); }
catch { console.error('无法读取主服务 TOML 配置，请检查配置后重试。'); process.exit(1); }
let databaseEnv;
try {
  databaseEnv = await prepareViewerDatabase({ root, viewers: metadata.viewers, env: process.env });
} catch (error) { console.error(error.message); process.exit(1); }
const child = spawn('python3', [join(root, 'scripts/rust_cache.py'), 'run', '--locked', '-p', 'meowlive-server', '--', '--config', configPath], {
  cwd: root, env: { ...process.env, ...databaseEnv }, stdio: 'inherit',
});
child.on('error', () => { console.error('无法启动主服务，请检查 Python 与 Rust 环境。'); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code ?? 1; });
