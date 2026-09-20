import { access, mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { randomBytes } from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execute = promisify(execFile);
const passwordNames = ['MEOWLIVE_POSTGRES_ADMIN_PASSWORD', 'MEOWLIVE_POSTGRES_APP_PASSWORD', 'MEOWLIVE_NEO4J_PASSWORD'];

async function exists(path) {
  try { await access(path); return true; }
  catch (error) { if (error.code === 'ENOENT') return false; throw error; }
}

// Only the server receives the application connection string. Never include
// Compose output or private credentials in a launcher response or diagnostic.
export async function prepareViewerDatabase({ root, viewers, env, run = execute }) {
  if (!viewers.enabled) return {};
  const name = viewers.database_url_env;
  if (env[name]) return {};
  if (name !== 'MEOWLIVE_DATABASE_URL') throw new Error('请为观众存储设置配置指定的数据库环境变量。');
  const directory = join(root, 'config/local');
  const path = join(directory, 'databases.env');
  if (!await exists(path)) {
    if (await exists(join(root, 'data/databases/postgres/PG_VERSION'))) {
      throw new Error('已有 PostgreSQL 数据但缺少数据库凭据，请恢复 config/local/databases.env 后重试。');
    }
    await mkdir(directory, { recursive: true, mode: 0o700 });
    const contents = passwordNames.map(key => `${key}=${randomBytes(32).toString('hex')}\n`).join('');
    try { await writeFile(path, contents, { flag: 'wx', mode: 0o600 }); }
    catch (error) { if (error.code !== 'EEXIST') throw new Error('无法保存本项目数据库凭据，请检查 config/local 的写入权限。'); }
  }
  let password;
  try {
    await run('docker', ['compose', '--env-file', path, '-f', join(root, 'config/databases.compose.yaml'),
      'up', '-d', '--wait', '--wait-timeout', '60', 'postgres'], {
      cwd: root, env,
      timeout: 90_000, maxBuffer: 64 * 1024,
    });
    // Compose owns dotenv interpolation (including $$ and quoted values).
    // Read the effective container value instead of reparsing it with Node.
    const { stdout } = await run('docker', ['inspect', 'meowlive2d-postgres', '--format', '{{json .Config.Env}}'], {
      cwd: root, env, timeout: 5_000, maxBuffer: 64 * 1024,
    });
    const prefix = 'MEOWLIVE_POSTGRES_APP_PASSWORD=';
    password = JSON.parse(stdout).find(value => value.startsWith(prefix))?.slice(prefix.length);
    if (!password) throw new Error('missing application password');
  } catch { throw new Error('PostgreSQL 启动失败，请确认 Docker 已运行且本项目数据库配置有效，然后重试开启主服务。'); }
  return { [name]: `postgresql://meowlive_app:${encodeURIComponent(password)}@127.0.0.1:25432/meowlive` };
}
