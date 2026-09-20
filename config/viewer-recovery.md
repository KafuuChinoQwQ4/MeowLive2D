# 观众数据备份与恢复

PostgreSQL 是身份、礼物、积分、记忆、关系及删除记录的权威来源。Neo4j 和向量是可重建副本。备份包含个人资料，放在私有、访问受限且有独立保留周期的位置；不放进 Git。

以下命令针对本项目 Compose 容器。先确认容器名为 `meowlive2d-postgres`，不要对其他项目数据库操作。凭据从容器自身私有环境读取，不出现在命令参数或日志中。把备份目录权限设为仅当前用户可读；文件名按实际时间命名。

```bash
install -d -m 700 data/backups/viewers
docker exec meowlive2d-postgres sh -c 'export PGPASSWORD="$POSTGRES_PASSWORD"; exec pg_dump -h 127.0.0.1 -U meowlive_admin -d meowlive --format=custom' > data/backups/viewers/meowlive.dump
chmod 600 data/backups/viewers/meowlive.dump
```

同时备份 `[viewers].receipt_directory` 中尚未提交的最小完成回执日志；恢复时保持原 scope 与 speech ID，服务会幂等重试。不要以新的时间或 ID 重写日志。

备份成功需检查命令退出码与 `pg_restore --list`，定期在全新的隔离数据库演练。不要将恢复直接覆盖仍在接收直播事件的数据库。停止本服务后，由数据库管理员创建空库，在该空库执行 `pg_restore --exit-on-error --dbname=<隔离恢复库>`；先核对迁移、身份数量、事件去重、积分流水和删除记录，再切换服务私有连接变量。不要同时让两个服务写同一逻辑 scope。

恢复保留对象所有者及权限，目标 PostgreSQL 必须先具备 `meowlive_admin` 和受限的 `meowlive_app` 角色；新服务器需由管理员安全创建角色和私有密码。不要用 `--no-owner --no-acl` 让业务表全部归管理账号所有。新库还需单独配置数据库权限和应用账号的查找路径（将下例 `viewer_restore` 换为实际恢复库名）：

```sql
REVOKE ALL ON DATABASE viewer_restore FROM PUBLIC;
GRANT CONNECT, TEMPORARY ON DATABASE viewer_restore TO meowlive_app;
ALTER ROLE meowlive_app IN DATABASE viewer_restore SET search_path = app, public;
```

使用应用账号连接恢复库，核对 `current_schema()` 为 `app`、能够读取 `_sqlx_migrations` 和 `viewers`，并在回滚事务中检查必要写入与去重约束。管理员检查业务表时显式指定 `app.` 或设置 `search_path=app,public`；数据库级角色设置不会因为恢复表数据而自动继承。

备份后的删除必须单独同步保存。关闭接收和后台 worker 后，从最新权威库导出 `memory_tombstones`、`memory_suppressions`，以及 `relationship_facts` 中 `deleted=true` 的 `(scope_id,id,version)`。这些删除资料必须比选定备份新，且与所恢复 scope 对应；如果无法取得后续删除资料，不应启用旧备份的记忆检索和图查询。

恢复后先在事务中把删除资料加载到临时表，执行：

1. 按 `(scope_id,memory_id)` 将对应记忆标记 `deleted=true`、`locked=true`，版本推进到大于旧备份和删除记录的最大值；合并 `memory_tombstones` 和全部同源 `memory_suppressions`。即使一条墓碑来自纠正操作，也保守屏蔽旧正文，随后由管理员恢复正确事实。
2. 删除这些记忆的 `memory_vectors`、`memory_embedding_jobs`，将旧 `memory_jobs` 标记失败并撤销租约。以 scope 推进 `memory_scopes.revision`。
3. 按关系删除资料将 `relationship_facts` 标记删除并推进版本；为所有关系（包括墓碑）重新排入 `relationship_outbox`，清空旧租约。记忆关联图失效触发器也会生成必要删除任务。
4. 保留 `viewer_events`、`gift_ledger`、`affinity_ledger` 的来源唯一键；不回放直播或重新奖励旧回执。

上述步骤需要数据库管理员核对加载的临时表名称、列及 scope 后执行，不提供直接清空生产库的脚本。删除资料也需纳入独立备份；仅保留旧全量备份不能满足后续删除。

服务启动后先保持 Agent 暂停，在认证管理面板检查记忆与关系，再执行“重建向量”和“重建图”。向量按有效正文、配置模型和维度重新生成；图按 PostgreSQL 事实及版本墓碑覆盖旧投影。后台任务故障修复后可执行“重试失败任务”。重建请求使用新 UUID 请求键；网络重试保留原键和理由。

恢复验收至少包括：旧事件重复接收不奖励、手动删除内容不再出现在 SQL/图/模型上下文、旧版本图写入不能复活墓碑、模型或 Neo4j 离线时仍可接收事件，以及重新配置后的租约能够完成。真实直播平台金额、实际模型提取质量和 Windows 播放需另行实机验收。
