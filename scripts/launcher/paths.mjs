import { homedir } from 'node:os';
import { isAbsolute, relative, resolve, sep } from 'node:path';

// Resolve only complete path values. Never interpolate paths into shell source or rewrite messages.
export function resolveConfiguredPath(root, value, home = homedir()) {
  if (value === '~') return home;
  if (value.startsWith('~/')) return resolve(home, value.slice(2));
  return resolve(root, value);
}

function inside(base, target) {
  const value = relative(base, target);
  return value === '' || (!isAbsolute(value) && value !== '..' && !value.startsWith(`..${sep}`));
}

// Public paths are display values; process arguments and file operations keep the original paths.
export function displayPath(value, root, home = homedir()) {
  if (!value) return '';
  const base = resolve(root ?? process.cwd());
  const target = resolveConfiguredPath(base, value, home);
  if (inside(base, target)) return `./${relative(base, target).split(sep).join('/')}`;
  if (inside(home, target)) return `~/${relative(home, target).split(sep).join('/')}`;
  return `./${relative(base, target).split(sep).join('/')}`;
}
