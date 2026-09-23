# Agent 运行、缓存与用量

在「LLM 接入」展开运行能力与用量区域。运行设置保存后用于下一次 Agent 决策，不需要重启；正在进行的决策保持开始时的配置。LLM 服务商、地址、模型及推理强度的修改仍按原有流程保存后重启主服务。

## 多厂商统一

application 的 `LanguageModel::turn` 接收统一工具、上下文和观察者，返回最终决策或工具请求。adapters 分别处理 OpenAI Chat Completions、OpenAI Responses、Anthropic Messages、Gemini generateContent/streamGenerateContent 的原生请求、工具续接、流式事件、结束状态和用量。工具续接保留原生 ID、思考签名及 Responses 的加密推理项目，服务层不改写这些字段。

流式接收用于首段延迟、输出字符进度和及时取消，不公开模型思考过程。未完整结束、被截断、被拒绝或格式不合法的输出不会进入播报。最终回复仍校验事件引用和业务 JSON；只有实际播放完成的回复进入对话历史。

`llm.max_response_bytes` 限制拼接后的完整 JSON；SSE 的原始传输预算为该值的 8 倍，且最多 8 MiB，用于容纳逐帧协议字段。两种预算都有硬上限。兼容 Chat 响应同时支持标准缓存明细与 DeepSeek 的 `prompt_cache_hit_tokens` 字段，优先采用标准字段。

## 推理强度

选择模型后可设置 `default / minimal / low / medium / high / xhigh / max / ultra`。保存的是用户选择的档位，实际请求使用当前具体型号与协议支持的档位；页面通过同一后端解析器展示实际结果。档位表示模型内部的相对计算投入，不表示跨厂商相等的 token 数、耗时或能力。

- `default` 不发送推理覆盖参数，保留厂商默认行为，不等于关闭思考。
- 型号支持所选档位时原样使用；超过最高档取最高档，低于最低档取最低档。
- 支持范围内缺少某档时，取不高于请求档位的最高支持档。例如仅支持 low / high / max 的型号，medium 使用 low，xhigh 使用 high，ultra 使用 max。
- 仅有预算参数的型号使用单独的预算映射。预算受用户设置的 `max_tokens` 上限约束，并为回答保留至少 512 token；不会为了高档位自动扩大上限。不够容纳厂商最低预算时，预览给出错误，保存和测试均拒绝该配置。
- 未知型号、未知协议组合或没有可调档位的型号保持原有请求，界面明确提示未覆盖。自定义兼容服务只对能力表中的已知精确型号启用映射；模型目录返回的 ID 本身不是推理能力证明。

`[llm].reasoning_effort` 默认为 `"default"`，旧 TOML 和本机保存文件缺少该字段时自动兼容。云端 `max_tokens` 可设置 64–65536；默认仍是 1024，本地模式仍不超过 1024。高推理强度可能需要提高输出上限和超时，否则厂商可能截断；这些限制仍由用户明确设置。连接测试与 Agent 的所有工具轮次复用同一档位解析与请求适配，一轮生成不会中途切档。

`POST /api/llm/reasoning` 接收 `provider / api_format / model / reasoning_effort / max_tokens`，不接收密钥、不访问供应商；返回所选档位、实际档位、可用档位、预算与说明。该接口沿用管理认证。能力表由代码维护，新增模型需核实官方参数后登记，不根据模型名中的版本数字推测能力。

当前能力登记覆盖已核实的 OpenAI、Claude、Gemini、DeepSeek、Grok 和 Kimi K3 型号。旧 Kimi K2.x 和目前核实的 GLM 接口只确认思考开关、没有可比的多档参数，因此不伪造档位。GPT-6 Astra 的工具调用应使用 OpenAI Responses 协议。

官方参数参考：[OpenAI reasoning](https://developers.openai.com/api/docs/guides/reasoning)、[Claude effort](https://platform.claude.com/docs/en/build-with-claude/effort)、[Claude extended thinking](https://platform.claude.com/docs/en/build-with-claude/extended-thinking)、[Gemini thinking](https://ai.google.dev/gemini-api/docs/generate-content/thinking)、[DeepSeek thinking](https://api-docs.deepseek.com/guides/thinking_mode/)、[Grok reasoning](https://docs.x.ai/developers/model-capabilities/text/reasoning)、[Kimi reasoning effort](https://platform.kimi.ai/docs/guide/use-reasoning-effort)。

## 缓存

固定的人设和工具定义、已完成历史放在动态事件与环境资料前。工具按名称排序；最后一轮保留定义并禁止继续选工具，避免无谓改变前缀。

| 协议 | 本项目的缓存处理 |
| --- | --- |
| OpenAI | 官方端点设置稳定的 `prompt_cache_key`；服务端自动缓存。未知兼容端点不添加专有缓存参数。 |
| Anthropic | 开启时为工具、系统提示和历史边界添加 `cache_control: ephemeral`，动态事件和环境放在其后。 |
| Gemini | 复用稳定前缀，使用模型支持的隐式缓存；读取 `cachedContentTokenCount`。未创建有存储费用的显式缓存资源。 |

开关控制本项目发送的缓存优化参数；关闭不能强制关闭厂商自动缓存。短上下文、模型不支持、历史前缀变化或厂商策略均可能不命中。缓存读写 token 是实际回报，缺失时显示未报告，不能根据“开启缓存”推断命中。

官方机制：[OpenAI](https://developers.openai.com/api/docs/guides/prompt-caching)、[Anthropic](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)、[Gemini](https://ai.google.dev/gemini-api/docs/generate-content/caching)。

## 搜索与感知工具

- `get_current_time`：真实 UTC 日期、时间和时间戳，可指定 UTC 分钟偏移；中国标准时间为 480。未知观众时区时不会据服务器时间推断所在地。
- `get_environment`：读取本次查询时的直播连接、事件接收数、播放队列、桌面连接及训练忙碌状态。初始决策也可附带这些动态资料。
- `get_obs_status`：经桌面执行端读取 OBS 是否连接、是否录制及当前场景；不会切场景或启动录制。
- `web_search`：检索公开问题、陌生词和需要最新信息的主题，最多返回 5 个标题、来源地址和摘要。摘要作为未受信资料，失败会反馈给模型用于澄清，不编造查询成功。

Brave 填写完整默认端点 `https://api.search.brave.com/res/v1/web/search` 和 API 密钥。SearXNG 填写实例的完整 `/search` 地址，并在实例 `settings.yml` 的 `search.formats` 中启用 `json`；本机实例可用 `http://127.0.0.1:端口/search`，公网端点须 HTTPS。更换搜索提供商或完整端点会清除旧目标的密钥；查询接口只回报密钥是否已配置。参考 [Brave API](https://api-dashboard.search.brave.com/app/documentation/web-search)、[SearXNG API](https://docs.searxng.org/dev/search_api)。

只有已启用且配置可用的搜索工具会提供给模型。工具调用上限默认 2 轮、每轮 4 次，可调整为 1–3 轮；单工具默认 6 秒，可调整为 1–8 秒。模型请求、重试和工具等待共用原 LLM 总超时。暂停、停止或连接断开会取消生成，过期上下文不会继续播报。

这些感知来自服务与连接状态；目前没有桌面截图视觉、摄像头或持续麦克风监听，也没有任意 shell、文件读取或操作系统控制工具。

## 调度观察、Trace 与 Turn

控制面板的独立「Agent 观察」页面直接读取 worker 使用的准入判断，可区分暂停、LLM 未配置、桌面执行端断开、资源或 GPU 忙、播放占用、决策/语音在途、冷却和没有可回应事件。这个状态是调度事实；没有待处理事件属于正常等待，不是错误。

worker 每次选中一个 Agent 工作就建立一条 Trace。同一工作中的每次真实模型 HTTP 调用都是一个 Turn：工具调用由产生它的 Turn 关联，带工具结果的后续请求和临时错误重试都会建立新的 Turn。时间线继续记录上下文准备、首段输出、最终决策、业务校验、语音排队、合成、等待播放和正在播放。静默决策通过校验后即可完成；有声回复只有收到执行端的终态回执后才完成。暂停、停止、设置变更、连接断开或主服务异常退出会把在途记录收束为取消、失败或进程中断。

完成历史保存在资源数据目录的 `agent-runtime/traces/`。活动记录使用 `active.json` 原子更新，完成记录追加到分段 JSONL；重启会把遗留活动记录标为进程中断。单条记录最多 256 KiB，单段最多 4 MiB，最多保留最近 1,000 条且总量不超过 32 MiB。超出单条容量时会优先裁剪来源链接并标记截断。启动时磁盘不可用或运行中写盘失败不会阻止 Agent 工作，页面会标记持久化不可用，并在内存中保留最近 100 条。损坏的既有活动或历史文件会在启动时明确报错。

Trace 只保存有界事件摘要、供应商/协议/模型、状态、耗时、用量、工具名和经过校验的公开来源。它不保存人设、完整提示词、对话或观众记忆正文、管理员备注、API 密钥、认证头、工具参数与返回正文、供应商原始响应或隐藏推理内容。该页面与 `GET /api/agent/scheduler`、`GET /api/agent/traces`、`GET /api/agent/traces/{id}` 均沿用控制面板管理认证，响应禁止缓存。

## 用量与估算费用

每次 Agent 模型调用单独记录，包括工具后续轮次与重试。记录状态区分运行中、完成、失败、取消、进程中断。失败前已收到的用量仍保留；取消时未收到完整回报不表示没有产生费用。首段延迟是本项目观察到的延迟。

输入总量包含缓存读取/写入，输出总量包含推理 token：Anthropic 的普通输入与两个缓存字段相加，Gemini 的候选输出与思考输出相加，OpenAI 直接使用其输入/输出总数。不能把分项再与总量相加。

价格按服务商、API 地址和模型匹配，以 USD/百万 token 填写。查询按当前价格重算历史，适合比较估算，不是保留历史价格的供应商发票。费用计算为：

`普通输入 × 输入单价 + 缓存读取 × 读取单价 + 缓存写入 × 写入单价 + 输出总量 × 输出单价`

结果换算为美元显示。推理已计入输出，不重复收费。未单独计费的缓存写入应填写普通输入单价；未报告缓存计数但相应价格与输入原价一致时，也能得到不依赖该计数的估算。缺少必要用量或单价则显示未知，汇总仅覆盖可估算部分。

支持日期、服务商和模型筛选，按模型聚合全部匹配历史，明细最多返回最近 200 条。聚合值是已报告部分的合计，零值不能证明所有调用都报告了该分项。查询数据仅覆盖本主服务的 Agent 生成；不包含模型连接测试、独立记忆提取/嵌入、网页搜索收费、厂商余额及其他程序的调用。

运行设置及账本保存在资源数据目录下 `agent-runtime/`，Unix 密钥文件权限为 0600。账本按大小分段追加；重启把未完成记录标为中断。运行中读取或落盘失败会显示持久化不可用，统计可能不完整，不能把缺失部分当作零费用。启动时若检测到已有账本或设置损坏，会明确报错并停止启动，避免静默重置记录。

接口：`GET/POST /api/agent/runtime`、`GET /api/agent/activity`、`GET /api/llm/usage?since_ms=...&until_ms=...&provider=...&model=...`，沿用控制面板管理认证。
