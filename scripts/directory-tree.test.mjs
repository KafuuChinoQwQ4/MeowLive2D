import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("./directory-tree.mjs", import.meta.url));
const gitignore = fileURLToPath(new URL("../.gitignore", import.meta.url));

function fixture(t) {
  const root = mkdtempSync(path.join(tmpdir(), "meowlive-directory-tree-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const catalog = {
    localDirectories: ["docs"],
    entries: {
      ".": "测试工程",
      docs: "本地文档独立索引",
      scripts: "开发工具",
      "scripts/directory-descriptions.json": "目录用途登记",
      src: "业务源码",
      "src/main.rs": "程序入口",
    },
  };
  const write = (name, content) => {
    mkdirSync(path.dirname(path.join(root, name)), { recursive: true });
    writeFileSync(path.join(root, name), content);
  };
  const saveCatalog = () => write("scripts/directory-descriptions.json", JSON.stringify(catalog));
  write("src/main.rs", "fn main() {}\n");
  saveCatalog();
  const run = (mode) => spawnSync(process.execPath, [script, mode, "--root", root], { encoding: "utf8" });
  const read = (name) => readFileSync(path.join(root, name), "utf8");
  const ok = (mode) => {
    const result = run(mode);
    assert.equal(result.status, 0, result.stderr);
  };
  return { root, catalog, write, read, saveCatalog, run, ok };
}

test("recursively generates annotated trees and check rejects an edited index", (t) => {
  const f = fixture(t);
  f.ok("--write");
  assert.match(f.read("DIRECTORY.md"), /main\.rs\s+# 程序入口/);
  assert.match(f.read("src/DIRECTORY.md"), /main\.rs\s+# 程序入口/);
  assert.match(f.read("scripts/DIRECTORY.md"), /directory-descriptions\.json/);
  f.ok("--check");
  f.write("src/DIRECTORY.md", "手动删掉了目录树\n");
  assert.notEqual(f.run("--check").status, 0);
  assert.equal(f.read("src/DIRECTORY.md"), "手动删掉了目录树\n", "check must not write");
  f.ok("--write");
  f.ok("--check");
});

test("content-only edits invalidate their directory and ancestor indexes", (t) => {
  const f = fixture(t);
  f.ok("--write");
  const rootBefore = f.read("DIRECTORY.md");
  const childBefore = f.read("src/DIRECTORY.md");
  f.write("src/main.rs", "fn main() { println!(\"changed\"); }\n");
  assert.notEqual(f.run("--check").status, 0);
  assert.equal(f.read("DIRECTORY.md"), rootBefore);
  f.ok("--write");
  assert.notEqual(f.read("src/DIRECTORY.md"), childBefore);
  assert.notEqual(f.read("DIRECTORY.md"), rootBefore);
  f.ok("--check");
});

test("added and removed files require matching purpose entries before writing", (t) => {
  const f = fixture(t);
  f.ok("--write");
  const before = f.read("DIRECTORY.md");
  f.write("src/worker.rs", "// worker\n");
  let result = f.run("--write");
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /src\/worker\.rs/);
  assert.equal(f.read("DIRECTORY.md"), before, "validation must precede writes");
  f.catalog.entries["src/worker.rs"] = "后台任务执行";
  f.saveCatalog();
  f.ok("--write");
  assert.match(f.read("src/DIRECTORY.md"), /worker\.rs\s+# 后台任务执行/);
  rmSync(path.join(f.root, "src/worker.rs"));
  result = f.run("--check");
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /src\/worker\.rs/);
  delete f.catalog.entries["src/worker.rs"];
  f.saveCatalog();
  f.ok("--write");
  assert.doesNotMatch(f.read("DIRECTORY.md"), /worker\.rs/);
  f.ok("--check");
});

test("excluded dependencies, outputs, local settings and symlinks are not traversed", (t) => {
  const f = fixture(t);
  for (const name of ["node_modules/pkg/file.js", "target/debug/output", "apps/desktop/dist/index.html", "apps/desktop/src-tauri/gen/schemas/desktop-schema.json", "apps/desktop/src-tauri/target/debug/output", ".git/config", "config/local/secret", ".env", "run.log", "scripts/__pycache__/runner.cpython-310.pyc", "scripts/old.pyc"]) {
    f.write(name, "not project source\n");
  }
  // These parents are genuine project directories even when their generated children are excluded.
  Object.assign(f.catalog.entries, { apps: "应用入口", "apps/desktop": "桌面应用", "apps/desktop/src-tauri": "桌面外壳", config: "配置" });
  symlinkSync(path.join(f.root, "node_modules"), path.join(f.root, "dependency-link"), "junction");
  f.catalog.entries["dependency-link"] = "依赖符号链接（不递归）";
  f.saveCatalog();
  f.ok("--write");
  assert.equal(existsSync(path.join(f.root, "node_modules/DIRECTORY.md")), false);
  assert.equal(existsSync(path.join(f.root, "target/DIRECTORY.md")), false);
  assert.equal(existsSync(path.join(f.root, "scripts/__pycache__/DIRECTORY.md")), false);
  assert.equal(existsSync(path.join(f.root, "config/local/DIRECTORY.md")), false);
  assert.equal(existsSync(path.join(f.root, "apps/desktop/src-tauri/gen/DIRECTORY.md")), false);
  assert.equal(existsSync(path.join(f.root, "apps/desktop/src-tauri/target/DIRECTORY.md")), false);
  assert.doesNotMatch(f.read("DIRECTORY.md"), /node_modules|run\.log|secret/);
  f.ok("--check");
});

test("private configuration variants and Python environments stay outside directory indexes", (t) => {
  const f = fixture(t);
  for (const name of [
    "config/launcher.local.json", "config/desktop.local.toml.bak",
    ".venv/Lib/site-packages/package/__init__.py", "scripts/extension.pyd",
  ]) {
    f.write(name, "local-only content\n");
  }
  Object.assign(f.catalog.entries, {
    config: "配置",
    "config/server.example.toml": "公开配置模板",
    ".env.example": "公开环境变量模板",
  });
  f.write("config/server.example.toml", "[server]\n");
  f.write(".env.example", "MEOWLIVE_LLM_API_KEY=\n");
  f.saveCatalog();
  f.ok("--write");
  assert.equal(existsSync(path.join(f.root, ".venv/DIRECTORY.md")), false);
  assert.doesNotMatch(f.read("DIRECTORY.md"), /launcher\.local|desktop\.local|extension\.pyd|\.venv/);
  assert.match(f.read("DIRECTORY.md"), /server\.example\.toml/);
  assert.match(f.read("DIRECTORY.md"), /\.env\.example/);
  f.ok("--check");
});

test("Git excludes private settings and generated files while keeping templates, icons and fixtures publishable", (t) => {
  const f = fixture(t);
  f.write(".gitignore", readFileSync(gitignore, "utf8"));
  const initialized = spawnSync("git", ["init", "--quiet", f.root], { encoding: "utf8" });
  assert.equal(initialized.status, 0, initialized.stderr);
  const privatePaths = [
    "config/local/llm-key.txt", "config/local/vts-token.json", "config/server.local.toml",
    "config/launcher.local.json", "config/desktop.local.toml.bak", ".env", ".env.development.local",
    "data/models/voice.ckpt", "data/resources/reference.wav", "data/resources/resources.sqlite3",
    "logs/repository-review/report.json", "docs/acceptance.md", "node_modules/dependency/index.js",
    "target/debug/server", "crates/tool/target/debug/tool", "apps/desktop/src-tauri/gen/schemas/desktop.json",
    "apps/desktop/dist/index.html", "apps/desktop/coverage/index.html", "apps/desktop/.vite/deps/react.js",
    ".venv/Lib/site-packages/package/__init__.py", "scripts/__pycache__/runner.pyc", "scripts/extension.pyd",
  ];
  const publicPaths = [
    ".env.example", "config/server.example.toml", "config/desktop.example.toml", "config/launcher.example.json",
    "apps/desktop/src-tauri/icons/icon.png", "apps/desktop/src-tauri/icons/icon.ico", "src/main.rs",
    "tests/fixtures/example.wav", "tests/fixtures/agent-events.json", "scripts/engine_workspace.py",
  ];
  const result = spawnSync("git", ["-C", f.root, "-c", "core.excludesFile=/dev/null", "check-ignore", "--no-index", "--stdin"], {
    encoding: "utf8", input: [...privatePaths, ...publicPaths].join("\n") + "\n",
  });
  assert.equal(result.status, 0, result.stderr);
  const ignored = new Set(result.stdout.trim().split("\n"));
  for (const name of privatePaths) assert.ok(ignored.has(name), `${name} must not be published`);
  for (const name of publicPaths) assert.ok(!ignored.has(name), `${name} must remain publishable`);
});

test("ignored local docs have complete indexes without making tracked indexes depend on them", (t) => {
  const f = fixture(t);
  f.ok("--write");
  const mainBefore = f.read("DIRECTORY.md");
  f.write("docs/plan.md", "# 本地计划\n");
  f.write("docs/directory-descriptions.json", JSON.stringify({ entries: {
    ".": "本地文档", "plan.md": "实施计划", "directory-descriptions.json": "本地用途登记",
  } }));
  f.ok("--write");
  assert.match(f.read("docs/DIRECTORY.md"), /plan\.md\s+# 实施计划/);
  assert.equal(f.read("DIRECTORY.md"), mainBefore);
  f.write("docs/plan.md", "# 修改后的本地计划\n");
  assert.notEqual(f.run("--check").status, 0);
  f.ok("--write");
  assert.equal(f.read("DIRECTORY.md"), mainBefore);
  rmSync(path.join(f.root, "docs"), { recursive: true });
  f.ok("--check");
});

test("indexes remain valid when the project checkout directory is renamed", (t) => {
  const f = fixture(t);
  f.ok("--write");
  const moved = `${f.root}-renamed`;
  renameSync(f.root, moved);
  t.after(() => rmSync(moved, { recursive: true, force: true }));
  const result = spawnSync(process.execPath, [script, "--check", "--root", moved], { encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
});

test("an index symlink never writes through to a missing target", (t) => {
  const f = fixture(t);
  const target = path.join(f.root, "must-not-be-created.md");
  symlinkSync(target, path.join(f.root, "DIRECTORY.md"), "file");
  const result = f.run("--write");
  assert.notEqual(result.status, 0);
  assert.equal(existsSync(target), false);
});
