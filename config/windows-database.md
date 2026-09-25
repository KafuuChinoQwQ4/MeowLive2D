# Windows App 数据库下载与启用

Windows 安装版默认不启用观众数据库，是为了让尚未安装 PostgreSQL 的用户先打开 App。数据库准备好后可以补齐观众档案、跨重启事件记录、陪伴积分和记忆存储。下载本身不代表功能已启用，必须完成账号初始化、连接配置和首次建表。

App 中的入口是 **环境与模型 → 数据库与可选功能**，主服务未就绪时也可查看。观众记录提示存储不可用时，提供前往该入口的链接。

## 下载来源

| 项目 | 官方来源 | 用途 |
| --- | --- | --- |
| Docker Desktop | [Windows 下载与安装](https://docs.docker.com/desktop/setup/install/windows-install/) | 推荐方式；按官方要求准备 WSL2，使用 Linux 容器 |
| PostgreSQL + pgvector | [pgvector 官方镜像与安装说明](https://github.com/pgvector/pgvector#docker) | 项目使用 `pgvector/pgvector:0.8.6-pg16-bookworm`，Compose 首次启动自动下载 |
| 原生 PostgreSQL | [Windows 官方下载页](https://www.postgresql.org/download/windows/) | 自行维护数据库时使用；还需单独安装 pgvector 扩展 |
| Neo4j Community | [官方下载中心](https://neo4j.com/deployment-center/) | 可选图投影，观众档案、积分和记忆无需安装 |
| 项目配置 | [项目源码 ZIP](https://github.com/KafuuChinoQwQ4/MeowLive2D/archive/refs/heads/main.zip) | 含 Compose 和初始化 SQL，无需编译代码即可准备数据库 |

PostgreSQL 普通安装器不包含本项目所需的 pgvector 初始化。[pgvector 官方说明](https://github.com/pgvector/pgvector#windows) 提供 Windows 编译安装方法；不想准备编译工具链时优先用上面的配套容器。链接核对日期：2026-09-25。

## 推荐：Docker Desktop + 项目配套数据库

### 1. 安装并启动 Docker Desktop

按官方下载页完成环境安装，选择 Linux 容器并等待引擎运行。在 PowerShell 执行 `docker version` 和 `docker compose version` 确认客户端与引擎可用。Docker Desktop 安装成功后仍需手动打开。

### 2. 解压项目，创建凭据并启动 PostgreSQL

把源码 ZIP 解压到固定目录，PowerShell 进入含 `config` 文件夹的项目根目录。这里不需要 Rust、Node.js，也不需要构建项目。

保留 `config/databases.compose.yaml` 与 `config/postgres-init.sql` 的相对位置。数据库密码位于 `config/local/databases.env`，数据位于 `data/databases/postgres`。以后继续使用这个目录；不要解压另一份并期待它自动找到旧数据。

```powershell
# 在下载并解压的 MeowLive2D 项目根目录运行（含 config 文件夹）。
$ErrorActionPreference = 'Stop'
$file = 'config/local/databases.env'
if (!(Test-Path $file)) {
  if (Test-Path 'data/databases/postgres/PG_VERSION') {
    throw '已有数据库但凭据文件缺失，请恢复原 databases.env，不要重新生成密码。'
  }
  New-Item -ItemType Directory -Force 'config/local' | Out-Null
  $rng = [Security.Cryptography.RandomNumberGenerator]::Create()
  try {
    $lines = foreach ($key in @('MEOWLIVE_POSTGRES_ADMIN_PASSWORD', 'MEOWLIVE_POSTGRES_APP_PASSWORD', 'MEOWLIVE_NEO4J_PASSWORD')) {
      $bytes = New-Object byte[] 32
      $rng.GetBytes($bytes)
      $key + '=' + ([BitConverter]::ToString($bytes)).Replace('-', '').ToLowerInvariant()
    }
    $path = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($file)
    $stream = [IO.File]::Open($path, [IO.FileMode]::CreateNew)
    $writer = [IO.StreamWriter]::new($stream, [Text.UTF8Encoding]::new($false))
    try { $writer.Write(($lines -join [Environment]::NewLine) + [Environment]::NewLine) } finally { $writer.Dispose() }
  } finally { $rng.Dispose() }
}
docker compose --env-file config/local/databases.env -f config/databases.compose.yaml up -d --wait --wait-timeout 180 postgres
if ($LASTEXITCODE -ne 0) { throw '数据库未就绪；请检查 Docker 状态、网络、端口和启动日志。' }
```

Compose 只启动 PostgreSQL，不启动 Neo4j。端口只绑定本机 `127.0.0.1:25432`。全新数据目录会执行初始化 SQL，创建 `meowlive` 数据库、`vector` 扩展、非超级用户 `meowlive_app` 及 `app` schema。已有文件和账号不会自动重置；更改 env 文件不会更改旧数据库密码。

### 3. 配置 Windows App 的连接变量

先确认上一步成功，再在同一 PowerShell 中执行下列命令。它会读取本项目容器中的应用账号密码，保存当前 Windows 用户的 `MEOWLIVE_DATABASE_URL`，不在终端打印连接密码。**已有自建数据库连接时跳过此段，保留自己的连接变量。**

```powershell
# 数据库成功启动后，在同一个 PowerShell 窗口执行。
$values = docker inspect meowlive2d-postgres --format '{{json .Config.Env}}'
if ($LASTEXITCODE -ne 0) { throw '未找到本项目 PostgreSQL，请先完成数据库启动。' }
$settings = $values | ConvertFrom-Json
$entry = @($settings | Where-Object { $_.StartsWith('MEOWLIVE_POSTGRES_APP_PASSWORD=') })
if ($entry.Count -ne 1) { throw '无法读取应用账号密码，请检查数据库初始化配置。' }
$password = $entry[0].Substring('MEOWLIVE_POSTGRES_APP_PASSWORD='.Length)
$url = 'postgresql://meowlive_app:' + [Uri]::EscapeDataString($password) + '@127.0.0.1:25432/meowlive'
[Environment]::SetEnvironmentVariable('MEOWLIVE_DATABASE_URL', $url, 'User')
$env:MEOWLIVE_DATABASE_URL = $url
Remove-Variable values, settings, entry, password, url
```

主服务只读取连接变量，不需要把管理账号交给 App。保存的 URL 是凭据，不要粘贴到截图、问题报告或公开日志。

### 4. 启用并重启 App

先关闭 MeowLive2D，再编辑 `%APPDATA%\io.meowlive.desktop\server\server.toml`，找到已有的 `[viewers]` 段，只调整如下字段，保留其他配置；不要添加重复段：

```toml
[viewers]
enabled = true
database_url_env = "MEOWLIVE_DATABASE_URL"
```

使用 `--config` 启动时，文件在桌面配置同目录的 `server/server.toml`，不一定是默认 AppData 目录。保留已有 `scope_id`，它是观众记录所属的稳定角色范围。

**注销并重新登录 Windows**，让开始菜单、桌面快捷方式读取刚保存的用户环境变量；再启动 Docker Desktop、确认容器运行，并打开 App。仅关闭再打开旧终端不保证环境变量刷新。当前 PowerShell 已设置进程变量，也可在这个窗口用安装后的 EXE 路径直接启动 App，免去本次注销。

### 5. 检查是否启用成功

等待主服务就绪，进入“观众记录”。能够显示空列表或已有记录、且没有“存储不可用”，表示连接和建表已成功；空库没有观众记录是正常的。首次启动会自动执行业务表迁移，不要手动导入其他项目的表。

日常先启动 Docker Desktop，再从原项目目录运行上述 Compose 启动命令，最后打开 MeowLive2D。App 自身不启动 Docker 或数据库容器。查看状态和停止数据库：

```powershell
docker compose --env-file config/local/databases.env -f config/databases.compose.yaml ps
docker compose --env-file config/local/databases.env -f config/databases.compose.yaml stop postgres
```

停止后数据保留。备份、恢复和数据删除见 [数据库恢复说明](viewer-recovery.md)。

## 已有原生 PostgreSQL 或远程数据库

确认数据库服务器已安装 pgvector。在全新、专供 MeowLive2D 的数据库上，用管理员的 psql 执行以下初始化；已有 MeowLive2D 库不要重新创建角色或覆盖密码：

```sql
CREATE DATABASE meowlive;
\connect meowlive
CREATE EXTENSION vector;
CREATE ROLE meowlive_app WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION;
\password meowlive_app
REVOKE ALL ON DATABASE meowlive FROM PUBLIC;
GRANT CONNECT, TEMPORARY ON DATABASE meowlive TO meowlive_app;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO meowlive_app;
CREATE SCHEMA app AUTHORIZATION meowlive_app;
ALTER ROLE meowlive_app IN DATABASE meowlive SET search_path = app, public;
```

`\password` 会交互询问应用账号密码。在 Windows 用户环境变量中设置 `MEOWLIVE_DATABASE_URL`，格式为 `postgresql://meowlive_app:<URL编码后的密码>@<地址>:<端口>/meowlive`。Docker 配套端口是 25432，原生 PostgreSQL 通常使用 5432，以安装时的设置为准。特殊字符必须 URL 编码，不直接粘贴明文密码到 URL。

再按上面的第 4–5 步启用和验证。使用远程数据库时按部署要求配置 TLS、允许的客户端地址与数据库认证；不要将端口向公网无条件开放。

## 错误排查与恢复临时模式

- “观众数据库环境变量未设置”：检查变量名称、当前 Windows 用户和 App 启动进程是否继承新变量。
- “观众数据库连接或迁移失败”：检查 Docker/数据库是否运行、端口、账号密码、`vector` 扩展、`app` schema 权限。`server.log` 位于桌面配置目录。
- 25432 端口冲突：先确认是否已有本项目数据库，避免另建同名容器或覆盖旧数据；自建部署修改端口时，连接变量同步修改。
- 已有数据但缺失凭据：恢复原 `config/local/databases.env` 或通过数据库管理员重置应用账号，不删除数据目录。
- 想先恢复其他功能：将同一配置的 `[viewers].enabled` 改回 `false` 后重新打开 App；如果自行启用了 `[memory]` 或 `[graph]`，同时关闭这些依赖观众存储的功能。已有数据库和记录不会被删除。

Neo4j 图投影、后台记忆提取/嵌入模型仍需独立配置，安装 PostgreSQL 不会自动开启模型调用。步骤见 [陪伴、记忆与图谱配置](README.md#陪伴记忆与图谱)。Windows 内置主服务的本地语音训练限制也不会因安装数据库而解除。
