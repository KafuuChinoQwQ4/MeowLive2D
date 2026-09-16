import { createHash } from "node:crypto";
import { existsSync, lstatSync, readFileSync, readdirSync, readlinkSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const indexName = "DIRECTORY.md";
const indexPurpose = "本目录递归目录树、文件用途与同步指纹（自动生成）";
const excludedDirectories = new Set([".git", "node_modules", "target", "dist", ".vite", "coverage", ".venv", "__pycache__"]);
const errors = [];
const outputs = [];

function excluded(relative, name) {
  return excludedDirectories.has(name)
    || ["data", "logs", "config/local"].includes(relative)
    || /(^|\/)src-tauri\/gen$/.test(relative)
    || (/^\.env(?:\.|$)/.test(name) && name !== ".env.example")
    || /\.(?:log|tsbuildinfo|py[cod])$/.test(name)
    || /^config\/[^/]+\.local\.[^/]+$/.test(relative)
    || [".DS_Store", "Thumbs.db"].includes(name);
}

function loadCatalog(filename) {
  const catalog = JSON.parse(readFileSync(filename, "utf8"));
  if (!catalog.entries || Array.isArray(catalog.entries) || typeof catalog.entries !== "object") {
    throw new Error(`${filename}: entries 必须是路径与用途的映射。`);
  }
  for (const [name, purpose] of Object.entries(catalog.entries)) {
    const validPath = name === "." || (!name.startsWith("/") && !name.includes("\\")
      && name.split("/").every((part) => part && part !== "." && part !== ".."));
    if (!validPath || /[\r\n\t]/.test(name) || typeof purpose !== "string" || !purpose.trim()
      || /[\r\n\t]/.test(purpose)) {
      throw new Error(`${filename}: 无效的路径或空白、多行用途说明：${name}`);
    }
  }
  return catalog;
}

function scan(root, catalog, boundaries = [], rootName = "MeowLive2D") {
  const used = new Set();
  function purposeFor(relative) {
    used.add(relative);
    const purpose = catalog.entries[relative];
    if (!purpose) errors.push(`缺少用途说明：${path.join(root, relative)}`);
    return purpose ?? "";
  }
  function visit(relative) {
    const fullPath = path.join(root, relative);
    const purpose = purposeFor(relative);
    const children = [];
    for (const entry of readdirSync(fullPath, { withFileTypes: true })) {
      const childPath = relative === "." ? entry.name : `${relative}/${entry.name}`;
      if (entry.name === indexName || excluded(childPath, entry.name)) continue;
      if (/[\r\n\t]/.test(entry.name)) throw new Error(`文件名不能包含换行或制表符：${childPath}`);
      if (relative === "." && boundaries.includes(entry.name)) continue;
      if (entry.isDirectory()) {
        children.push(visit(childPath));
      } else if (entry.isFile() || entry.isSymbolicLink()) {
        children.push({
          name: entry.name, relative: childPath, fullPath: path.join(root, childPath),
          type: entry.isSymbolicLink() ? "symlink" : "file", purpose: purposeFor(childPath),
        });
      } else {
        throw new Error(`不支持的文件类型：${childPath}`);
      }
    }
    if (relative === ".") {
      for (const boundary of boundaries) {
        children.push({ name: boundary, type: "boundary", relative: boundary, purpose: purposeFor(boundary) });
      }
    }
    children.push({ name: indexName, type: "generated", purpose: indexPurpose });
    children.sort((a, b) => {
      const aDir = ["directory", "boundary"].includes(a.type);
      const bDir = ["directory", "boundary"].includes(b.type);
      return Number(bDir) - Number(aDir) || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
    });
    const name = relative === "." ? rootName : path.basename(fullPath);
    return { name, relative, fullPath, type: "directory", purpose, children };
  }
  const tree = visit(".");
  for (const name of Object.keys(catalog.entries)) {
    if (!used.has(name)) errors.push(`用途说明已无对应文件或目录：${path.join(root, name)}`);
  }
  return tree;
}

function collect(tree, catalogPath) {
  const hash = createHash("sha256");
  function fingerprint(node) {
    // Generated indexes never hash themselves; local boundaries do not hash ignored files.
    hash.update(JSON.stringify([node.relative ?? node.name, node.type, node.purpose]) + "\n");
    if (node.type === "file") hash.update(readFileSync(node.fullPath));
    if (node.type === "symlink") hash.update(readlinkSync(node.fullPath));
    if (node.children) node.children.forEach(fingerprint);
  }
  fingerprint(tree);
  const lines = [`${tree.name}/  # ${tree.purpose}`];
  function render(children, prefix) {
    children.forEach((node, index) => {
      const last = index === children.length - 1;
      const suffix = ["directory", "boundary"].includes(node.type) ? "/" : "";
      lines.push(`${prefix}${last ? "└──" : "├──"} ${node.name}${suffix}  # ${node.purpose}`);
      if (node.children) render(node.children, prefix + (last ? "    " : "│   "));
    });
  }
  render(tree.children, "");
  const childLinks = tree.children
    .filter((node) => ["directory", "boundary"].includes(node.type))
    .map((node) => `- [${node.name}/](${encodeURIComponent(node.name)}/${indexName})：${node.purpose}`);
  const content = [
    `# ${tree.name} 目录索引`, "", tree.purpose, "",
    "本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。", "",
    "```text", ...lines, "```", "",
    ...(childLinks.length ? ["可继续查看各子目录的索引：", "", ...childLinks, ""] : []),
    `用途说明源：\`${catalogPath}\`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。`, "",
    "已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。", "",
    `<!-- directory-tree-sha256: ${hash.digest("hex")} -->`, "",
  ].join("\n");
  const filename = path.join(tree.fullPath, indexName);
  const existing = lstatSync(filename, { throwIfNoEntry: false });
  if (existing && !existing.isFile()) {
    errors.push(`索引路径必须是普通文件：${filename}`);
  }
  outputs.push({ filename, content });
  for (const child of tree.children) if (child.type === "directory") collect(child, catalogPath);
}

function main() {
  const [mode, rootFlag, rootValue, ...extra] = process.argv.slice(2);
  if (!["--write", "--check"].includes(mode) || extra.length
    || (rootFlag !== undefined && (rootFlag !== "--root" || !rootValue))) {
    throw new Error("用法：node scripts/directory-tree.mjs --write|--check [--root 项目路径]");
  }
  const root = rootValue ? path.resolve(rootValue) : fileURLToPath(new URL("../", import.meta.url));
  const catalogPath = "scripts/directory-descriptions.json";
  const catalog = loadCatalog(path.join(root, catalogPath));
  const boundaries = catalog.localDirectories ?? [];
  if (!Array.isArray(boundaries) || boundaries.some((name) => typeof name !== "string"
    || !/^[\w.-]+$/.test(name) || [".", ".."].includes(name) || excluded(name, name))
    || new Set(boundaries).size !== boundaries.length) {
    throw new Error("localDirectories 必须是唯一、未排除的顶层目录名。");
  }
  collect(scan(root, catalog, boundaries), catalogPath);
  for (const boundary of boundaries) {
    const localRoot = path.join(root, boundary);
    if (!existsSync(localRoot)) continue;
    if (!lstatSync(localRoot).isDirectory()) throw new Error(`本地索引边界必须是普通目录：${boundary}`);
    const localCatalogPath = "directory-descriptions.json";
    collect(scan(localRoot, loadCatalog(path.join(localRoot, localCatalogPath)), [], boundary), `${boundary}/${localCatalogPath}`);
  }
  if (errors.length) throw new Error(errors.join("\n"));
  const stale = outputs.filter(({ filename, content }) =>
    !existsSync(filename) || readFileSync(filename, "utf8") !== content);
  if (mode === "--check" && stale.length) {
    throw new Error(`目录索引缺失或过期，请运行 npm run tree:update：\n${stale.map(({ filename }) => filename).join("\n")}`);
  }
  if (mode === "--write") for (const { filename, content } of stale) writeFileSync(filename, content);
  console.log(mode === "--write"
    ? `已更新 ${stale.length} 份目录索引，共覆盖 ${outputs.length} 个目录。`
    : `目录索引检查通过，共 ${outputs.length} 个目录。`);
}

try {
  main();
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
