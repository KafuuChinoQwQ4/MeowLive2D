# 测试分工

`npm test` 执行工具、Rust workspace 与前端测试；`npm run check` 还包含格式、编译、契约、目录索引与类型验证。

| 测试位置 | 内容 |
| --- | --- |
| `crates/domain/tests/` | 文本与音色标识不变量 |
| `crates/application/tests/` | FIFO、容量、回执、停止、断线与历史 |
| `crates/adapters/tests/` | 受控 HTTP 引擎、超时、限量、WAV 完整性 |
| `crates/protocol/tests/` | JSON DTO 与二进制 PCM 兼容 |
| `crates/desktop-runtime/tests/` | 播放状态机、设备时钟/格式转换、双 WS 与 CLI |
| `apps/server/tests/` | 配置、HTTP、配对、取消、回执及真实运行时联调 |
| `apps/desktop/src/**/*.test.ts*` | services、组件、轮询与用户操作 |
| `scripts/directory-tree.test.mjs` | 目录工具与排除边界 |

按职责与场景分文件，公共夹具放 `support/` 或前端 `src/test/`；不在正式源码内堆放测试。根 tests 是分工说明，不会被 Cargo 自动执行。模拟服务、静音后端通过不代表 GPU、Windows、VTS 或 OBS 验收。
