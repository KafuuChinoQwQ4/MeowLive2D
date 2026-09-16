# MeowLive2D 项目协作规则

本文件适用于本目录及所有子目录。

## 目录索引必须同步维护

这是用户明确要求的长期约定：每个受维护目录都必须有 `DIRECTORY.md`，其中递归列出目录树，并在每个文件及子目录旁标明大致用途。

每次新增、修改、删除或移动文件、目录，都必须在同一次工作中完成以下步骤：

1. 新增文件或目录：在 `scripts/directory-descriptions.json` 的 `entries` 中登记相对项目根目录的路径与中文用途。
2. 修改已有文件：核对用途是否变化；变化时同步用途描述，未变化时保留原描述。
3. 删除或移动文件、目录：删除旧用途条目，登记新路径；删除目录时一并移除其全部旧条目。
4. 运行 `npm run tree:update`，更新所有受影响的 `DIRECTORY.md`，包括祖先目录中的递归目录树和内容指纹。
5. 运行 `npm run tree:check`；检查通过后再交付。`npm run check` 已包含这项检查。

`DIRECTORY.md` 由脚本生成，不直接编辑。用途说明是人工维护的事实；脚本不会自动理解代码变化。即便目录结构未变，内容指纹也会要求在修改文件后刷新索引。

## 本地文档与排除范围

- `docs/` 继续保持 Git 忽略。其用途登记在 `docs/directory-descriptions.json`，路径相对于 `docs/`；同一个生成命令递归更新本地文档索引。
- 根索引只记录 `docs/` 这个本地目录边界，完整文档树在 `docs/DIRECTORY.md`。这样本地文档增删不会导致没有 `docs/` 的检出无法通过检查。
- 新建本地 `docs/` 时，同时建立上述登记文件；没有 `docs/` 的环境直接跳过本地文档检查。
- 依赖、构建输出、运行数据和私有配置不生成索引：`node_modules/`、`target/`、`dist/`、`.vite/`、`coverage/`、`.venv/`、`__pycache__/`、`.git/`、根 `data/` 和 `logs/`、`config/local/`、`config/*.local.*`、`.env*`（保留 `.env.example`）、日志与 TypeScript 构建缓存等。精确规则在 `scripts/directory-tree.mjs`。
- 符号链接可以记录用途，但不递归其目标。不得向依赖目录或项目外路径写入索引。
- 新增一类自动生成目录时，同时更新忽略规则、索引排除规则和相应测试。

## 模块边界

保持根 README 中的单向依赖：`application → domain`，外部适配器实现 application 的接口，跨端契约集中在 protocol，Windows 执行库独立于 Tauri 外壳。前端通过 services 访问外部能力。

保留无关的已有改动。只修改本项目内获授权的内容，不操作父目录 Git 仓库。

## 验证

- 目录与文件改动：`npm run tree:update`、`npm run tree:check`。
- 索引脚本或排除规则改动：额外执行 `npm run test:tooling`。
- Rust / TypeScript 代码改动：根据影响执行 `npm run check`、`npm run build` 和相应测试；如实说明未验证的 Windows、模型或直播行为。
