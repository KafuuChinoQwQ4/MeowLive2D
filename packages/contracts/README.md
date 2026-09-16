# 前端协议类型

本包是前端访问跨进程类型及协议版本常量的唯一出口，不提供运行时请求客户端。

权威定义属于 `crates/protocol`。`src/index.ts` 由 Rust 的 `ts-rs` 声明生成：

```sh
cargo run -p meowlive-protocol --bin export-types
cargo run -p meowlive-protocol --bin export-types -- --check
```

请勿直接修改生成文件。修改 Rust DTO 后重新生成，并将生成结果一致性检查纳入交付验证。

前端使用 `import type` 消费 DTO，使用普通 `import` 消费 `PROTOCOL_VERSION`。不要在 feature 目录手写另一份 Rust 消息结构；运行时收到的数据仍须在服务客户端边界验证，TypeScript 类型不能替代网络数据校验。
