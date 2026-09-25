import { useState, type ReactNode } from "react";
import { dependencySources, isDesktopApp, openDependencyPage, type DependencyId } from "../../services/dependencies";
import "./database-setup.css";

const prepareCredentials = String.raw`# 在下载并解压的 MeowLive2D 项目根目录运行（含 config 文件夹）。
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
if ($LASTEXITCODE -ne 0) { throw '数据库未就绪；请检查 Docker 状态、网络、端口和启动日志。' }`;

const configureConnection = String.raw`# 数据库成功启动后，在同一个 PowerShell 窗口执行。
$values = docker inspect meowlive2d-postgres --format '{{json .Config.Env}}'
if ($LASTEXITCODE -ne 0) { throw '未找到本项目 PostgreSQL，请先完成数据库启动。' }
$settings = $values | ConvertFrom-Json
$entry = @($settings | Where-Object { $_.StartsWith('MEOWLIVE_POSTGRES_APP_PASSWORD=') })
if ($entry.Count -ne 1) { throw '无法读取应用账号密码，请检查数据库初始化配置。' }
$password = $entry[0].Substring('MEOWLIVE_POSTGRES_APP_PASSWORD='.Length)
$url = 'postgresql://meowlive_app:' + [Uri]::EscapeDataString($password) + '@127.0.0.1:25432/meowlive'
[Environment]::SetEnvironmentVariable('MEOWLIVE_DATABASE_URL', $url, 'User')
$env:MEOWLIVE_DATABASE_URL = $url
Remove-Variable values, settings, entry, password, url`;

function SourceLink({ id, children }: { id: DependencyId; children: ReactNode }) {
  const [error, setError] = useState(false);
  return <span className="database-source"><a href={dependencySources[id]} target="_blank" rel="noopener noreferrer"
    onClick={event => {
      if (!isDesktopApp()) return;
      event.preventDefault();
      setError(false);
      void openDependencyPage(id).catch(() => setError(true));
    }}>{children}</a>{error && <span role="alert">无法打开浏览器，请复制地址：<code>{dependencySources[id]}</code></span>}</span>;
}

export function DatabaseSetupPanel() {
  return <section className="panel database-setup" aria-labelledby="database-setup-heading">
    <div className="section-title"><h2 id="database-setup-heading">数据库与可选功能</h2><span className="field-hint">安装与启用指引</span></div>
    <p>观众档案、事件持久化、陪伴积分和记忆需要 PostgreSQL 与 pgvector。Windows 安装版首次默认关闭这些功能，准备好数据库后可以启用。</p>
    <p className="muted">此处提供官方来源和操作步骤，不代表已检测到安装完成或连接成功。</p>
    <div className="panel-grid">
      <article className="connection-card"><div><h3>PostgreSQL + pgvector</h3><p>推荐通过 Docker 安装项目配套数据库，包含所需扩展。</p>
        <SourceLink id="docker">下载 Docker Desktop（Windows）</SourceLink>
        <SourceLink id="pgvector">查看 pgvector 官方镜像与安装说明</SourceLink>
        <p className="field-hint">项目使用 PostgreSQL 16 + pgvector 0.8.6，连接端口为 25432。</p>
      </div></article>
      <article className="connection-card"><div><h3>已有数据库或需要图谱</h3>
        <SourceLink id="postgres">PostgreSQL Windows 官方下载</SourceLink>
        <p>原生安装 PostgreSQL 后，还要按 pgvector 官方说明安装扩展、创建应用账号和数据库。</p>
        <SourceLink id="neo4j">Neo4j 官方下载（可选）</SourceLink>
        <p className="field-hint">Neo4j 仅用于可选图投影；观众档案、积分、记忆不要求安装它。后台记忆提取模型也需单独配置。</p>
      </div></article>
    </div>
    <details className="database-instructions"><summary>Windows 安装版：补齐数据库并启用观众功能</summary>
      <ol>
        <li><strong>安装并启动 Docker Desktop。</strong><p>按官方页面完成 WSL2 环境准备，使用 Linux 容器，等待 Docker 引擎运行。</p></li>
        <li><strong>取得项目数据库配置。</strong><p><SourceLink id="project">下载项目配置与源码 ZIP</SourceLink>解压到固定目录。用 PowerShell 进入含 <code>config</code> 文件夹的项目根目录；下列命令只需要 Docker，不需要编译项目。</p>
          <p>保留 <code>config/databases.compose.yaml</code>、<code>config/postgres-init.sql</code> 的相对位置。密码存放于 <code>config/local/databases.env</code>，数据写入 <code>data/databases/postgres</code>，以后沿用此目录。</p></li>
        <li><strong>创建独立凭据并启动数据库。</strong><p>首次会拉取 <code>pgvector/pgvector:0.8.6-pg16-bookworm</code>。已有凭据保持原样，启动过程会建立扩展、应用账号和 schema。</p><pre><code>{prepareCredentials}</code></pre></li>
        <li><strong>配置 App 的数据库连接。</strong><p>下列命令保存当前 Windows 用户的连接变量，不输出密码。它会替换该用户已有的 <code>MEOWLIVE_DATABASE_URL</code>；已有自建数据库时跳过此步，继续使用自己的连接。</p><pre><code>{configureConnection}</code></pre></li>
        <li><strong>开启观众存储。</strong><p>关闭 MeowLive2D，打开 <code>{String.raw`%APPDATA%\io.meowlive.desktop\server\server.toml`}</code>，将已有 <code>[viewers]</code> 段改为以下内容，不要添加重复段。已有 <code>scope_id</code> 等其他设置保留。使用 <code>--config</code> 时，修改桌面配置同目录下的 <code>server/server.toml</code>。</p>
          <pre><code>{'[viewers]\nenabled = true\ndatabase_url_env = "MEOWLIVE_DATABASE_URL"'}</code></pre>
          <p>注销并重新登录 Windows，让开始菜单和桌面快捷方式读取新变量；再启动 Docker Desktop 和 MeowLive2D。当前 PowerShell 已有新变量，也可从这个窗口直接启动已安装的 App。</p></li>
        <li><strong>确认功能已启用。</strong><p>等待主服务就绪，再进入“观众记录”。显示空列表或已有记录、且没有“存储不可用”提示，才表示连接和建表成功；刚安装时没有观众记录是正常的。</p></li>
      </ol>
      <p className="availability-note">App 不负责启动 Docker。以后先启动 Docker Desktop，并在原项目目录运行上述 Compose 启动命令，再打开 App。停止容器会保留数据；不要通过删除数据目录来修复密码问题。</p>
    </details>
    <details className="database-instructions"><summary>Linux / WSL 源码启动器：准备和连接数据库</summary>
      <ol><li>安装并启动 Docker Engine；若使用 Docker Desktop，启用当前 WSL 发行版的集成。</li>
        <li>保留 <code>config/server.local.toml</code> 中的 <code>[viewers] enabled = true</code>，从项目目录运行 <code>./launchers/start.sh</code>。</li>
        <li>启动器自动生成本项目凭据并启动 PostgreSQL。已有 <code>MEOWLIVE_DATABASE_URL</code> 时直接连接指定数据库，不管理本地容器。</li>
        <li>等待主服务就绪，在“观众记录”确认读取成功。配置与备份说明见项目 <code>config/README.md</code> 和 <code>config/viewer-recovery.md</code>。</li></ol>
    </details>
    <details className="database-instructions"><summary>连接失败、已安装原生 PostgreSQL，或想先恢复临时模式</summary>
      <p>先检查 Docker 是否运行、25432 端口是否占用、应用账号密码是否匹配，以及数据库中是否已启用 vector 扩展。普通 PostgreSQL 安装器不会替本项目初始化 pgvector 和应用账号。</p>
      <p>自建数据库请按项目 <code>config/windows-database.md</code> 创建受限应用账号、<code>app</code> schema 和连接变量。主服务启动时会自动创建业务表。</p>
      <p>“环境与模型”在主服务失败时仍可访问。需要先恢复其他功能时，将同一配置的 <code>[viewers] enabled</code> 改回 <code>false</code>；如果启用了 <code>[memory]</code> 或 <code>[graph]</code>，同时将它们的 <code>enabled</code> 关闭，再重新打开 App。数据库与已有记录不会被删除。</p>
    </details>
  </section>;
}
