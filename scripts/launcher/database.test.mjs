import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, writeFile, rm, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { parseEnv } from 'node:util';
import { prepareViewerDatabase } from './database.mjs';

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'meow-database-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  return { root, viewers: { enabled: true, database_url_env: 'MEOWLIVE_DATABASE_URL' }, env: {} };
}

test('first start creates private credentials, starts only project PostgreSQL and reuses the credentials', async t => {
  const options = await fixture(t);
  const calls = [];
  const run = async (...args) => {
    calls.push(args);
    const passwords = parseEnv(await readFile(join(options.root, 'config/local/databases.env'), 'utf8'));
    return { stdout: JSON.stringify(Object.entries(passwords).map(([key, value]) => `${key}=${value}`)) };
  };
  const first = await prepareViewerDatabase({ ...options, run });
  const path = join(options.root, 'config/local/databases.env');
  const contents = await readFile(path, 'utf8');
  const passwords = parseEnv(contents);
  assert.equal(new Set(Object.values(passwords)).size, 3);
  assert.ok(Object.values(passwords).every(value => /^[a-f0-9]{64}$/.test(value)));
  assert.equal((await stat(path)).mode & 0o777, 0o600);
  assert.equal(calls[0][0], 'docker');
  assert.equal(calls[0][1].at(-1), 'postgres');
  assert.ok(calls[0][1].includes(join(options.root, 'config/databases.compose.yaml')));
  assert.equal(calls[0][2].env.MEOWLIVE_POSTGRES_APP_PASSWORD, undefined);
  assert.equal(first.MEOWLIVE_DATABASE_URL, `postgresql://meowlive_app:${passwords.MEOWLIVE_POSTGRES_APP_PASSWORD}@127.0.0.1:25432/meowlive`);
  assert.deepEqual(await prepareViewerDatabase({ ...options, run }), first);
  assert.equal(await readFile(path, 'utf8'), contents);
  assert.equal(calls.length, 4);
});

test('disabled storage and explicit external connections do not touch Docker or local files', async t => {
  const options = await fixture(t);
  const run = async () => assert.fail('unexpected Docker invocation');
  assert.deepEqual(await prepareViewerDatabase({ ...options, viewers: { ...options.viewers, enabled: false }, run }), {});
  assert.deepEqual(await prepareViewerDatabase({ ...options, env: { MEOWLIVE_DATABASE_URL: 'postgresql://external' }, run }), {});
  assert.deepEqual(await prepareViewerDatabase({ ...options, viewers: { enabled: true, database_url_env: 'CUSTOM_DB' }, env: { CUSTOM_DB: 'postgresql://custom' }, run }), {});
  await assert.rejects(stat(join(options.root, 'config/local')), { code: 'ENOENT' });
  await assert.rejects(prepareViewerDatabase({ ...options, viewers: { enabled: true, database_url_env: 'CUSTOM_DB' }, run }), /环境变量/);
});

test('existing application passwords are URL encoded and Docker errors never expose credentials', async t => {
  const options = await fixture(t);
  await mkdir(join(options.root, 'config/local'), { recursive: true });
  await writeFile(join(options.root, 'config/local/databases.env'), 'MEOWLIVE_POSTGRES_ADMIN_PASSWORD=admin-secret\nMEOWLIVE_POSTGRES_APP_PASSWORD="app:p@ss/$word#"\nMEOWLIVE_NEO4J_PASSWORD=graph-secret\n');
  const result = await prepareViewerDatabase({ ...options, run: async () => ({ stdout: JSON.stringify(['MEOWLIVE_POSTGRES_APP_PASSWORD=app:p@ss/$word#']) }) });
  const url = new URL(result.MEOWLIVE_DATABASE_URL);
  assert.equal(decodeURIComponent(url.password), 'app:p@ss/$word#');
  assert.equal(url.username, 'meowlive_app');
  await assert.rejects(prepareViewerDatabase({ ...options, run: async () => { throw new Error('private app:p@ss/$word#'); } }), error => {
    assert.match(error.message, /PostgreSQL 启动失败/);
    assert.equal(error.message.includes('app:p@ss'), false);
    return true;
  });
});

test('uses the effective Compose password instead of changing existing dollar escapes', async t => {
  const options = await fixture(t);
  await mkdir(join(options.root, 'config/local'), { recursive: true });
  await writeFile(join(options.root, 'config/local/databases.env'), 'MEOWLIVE_POSTGRES_ADMIN_PASSWORD=admin-secret\nMEOWLIVE_POSTGRES_APP_PASSWORD=demo$$suffix\nMEOWLIVE_NEO4J_PASSWORD=graph-secret\n');
  const result = await prepareViewerDatabase({ ...options, run: async (_command, args, invocation) => {
    assert.equal(invocation.env.MEOWLIVE_POSTGRES_APP_PASSWORD, undefined);
    if (args[0] === 'compose') return { stdout: '' };
    assert.deepEqual(args, ['inspect', 'meowlive2d-postgres', '--format', '{{json .Config.Env}}']);
    return { stdout: JSON.stringify(['MEOWLIVE_POSTGRES_APP_PASSWORD=demo$suffix']) };
  } });
  assert.equal(decodeURIComponent(new URL(result.MEOWLIVE_DATABASE_URL).password), 'demo$suffix');
});

test('missing credentials never regenerate passwords for an existing data directory', async t => {
  const options = await fixture(t);
  await mkdir(join(options.root, 'data/databases/postgres'), { recursive: true });
  await writeFile(join(options.root, 'data/databases/postgres/PG_VERSION'), '16');
  await assert.rejects(prepareViewerDatabase({ ...options, run: async () => assert.fail('unexpected Docker invocation') }), /恢复.*databases.env/);
  await assert.rejects(stat(join(options.root, 'config/local/databases.env')), { code: 'ENOENT' });
});
