import test from 'node:test';
import assert from 'node:assert/strict';
import { displayPath, resolveConfiguredPath } from './paths.mjs';

for (const [root, home, input, expected] of [
  ['/home/alice/projects/MeowLive2D', '/home/alice', '/home/alice/projects/MeowLive2D/logs/server.log', './logs/server.log'],
  ['/srv/live', '/home/bob', '/srv/live/data/models/a b', './data/models/a b'],
  ['/srv/live', '/home/bob', '/home/bob/.cache/models', '~/.cache/models'],
  ['/srv/live', '/home/bob', '/srv/live-other/models', './../live-other/models'],
  ['/srv/live', '/home/bob', '/home/bobby/models', './../../home/bobby/models'],
  ['/srv/live', '/home/bob', '/opt/models', './../../opt/models'],
  ['/srv/live', '/home/bob', '/srv/live', './'],
]) {
  test(`portable path ${input} retains its exact filesystem location`, () => {
    assert.equal(displayPath(input, root, home), expected);
    assert.equal(resolveConfiguredPath(root, expected, home), input);
  });
}

test('relative settings follow each checkout and home, without shell expansion', () => {
  assert.equal(resolveConfiguredPath('/srv/live', 'data/engine', '/home/bob'), '/srv/live/data/engine');
  assert.equal(resolveConfiguredPath('/srv/live', '~/models/GPT-SoVITS', '/home/bob'), '/home/bob/models/GPT-SoVITS');
  assert.equal(resolveConfiguredPath('/srv/live', '~', '/home/bob'), '/home/bob');
  assert.equal(resolveConfiguredPath('/srv/live', 'models/$(literal)', '/home/bob'), '/srv/live/models/$(literal)');
});
