# Icon Set

一个使用 `SvelteKit + Axum + Postgres` 构建的图标集合管理应用。公开页面展示示例图标集合；登录后的用户可以配置自己的 GitHub 图标仓库，后台数据按用户隔离。

源码仓库：[sevenzx/icon-set](https://github.com/sevenzx/icon-set)

## 功能

- 按 set 管理不同图标库，例如 `emby`、`sub`
- 未登录用户只看到内置演示集合，不展示后台入口
- GitHub OAuth 登录，后台入口按登录用户隔离
- 每个登录用户独立配置 GitHub owner、repo、branch 和 token
- 用户 GitHub Token 使用服务端 `ENCRYPTION_KEY` 加密后存入 Postgres
- 前台支持搜索图标、复制图标 Raw URL、复制集合 Manifest URL、查看大图和分页
- 后台支持创建 set、编辑集合信息、上传图片、批量上传图片或 zip 压缩包
- 批量上传总体积限制为 10MB，图片命名冲突会自动追加 `_01`、`_02`
- 上传图片时会在当前 set 内校验 MD5，避免同一集合重复收录相同图片
- 后台写接口同时校验 session cookie 和登录后签发的 `X-Admin-Token`
- 后台操作带同源 CSRF 校验、短时防重复提交和审计日志
- 删除集合和删除图标使用二段式确认弹窗

## 数据流

- 公开 API：返回内置演示数据，用于首页和未登录浏览，不读取真实 GitHub 仓库。
- 登录 API：通过 GitHub OAuth 建立 session，session 存在 Postgres。
- 后台 API：根据当前登录用户读取 `repo_config`，解密该用户的 GitHub Token 后写入对应仓库。
- 前端不会接触 GitHub 写权限 token，后台接口只识别当前登录 session。

## GitHub 仓库结构

后端会在配置的 GitHub 仓库里维护下面的结构：

```txt
sets.json
sets/
  emby/
    manifest.json
    icons/
      emby.png
  sub/
    manifest.json
    icons/
      netflix.png
```

`manifest.json` 示例：

```json
{
  "id": "emby",
  "name": "Emby图标库",
  "description": "",
  "icons": [
    {
      "id": "uuid",
      "name": "Emby",
      "path": "sets/emby/icons/emby.png",
      "url": "https://raw.githubusercontent.com/owner/repo/main/sets/emby/icons/emby.png",
      "md5": "image-content-md5"
    }
  ],
  "updated_at": "2026-05-29T00:00:00Z"
}
```

## 数据库

后端启动时会自动创建所需 Postgres 表。表名使用单数：

```txt
app_user
oauth_account
session
oauth_state
repo_config
audit_log
```

下面这些字段会使用 `ENCRYPTION_KEY` 加密后存储：

- `app_user.display_name`
- `app_user.email`
- `oauth_account.login`
- `repo_config.github_owner`
- `repo_config.github_repo`
- `repo_config.github_branch`
- `repo_config.github_token_ciphertext`

`oauth_account.provider_user_id` 会保留明文，用于稳定识别同一个 GitHub OAuth 账号。`ENCRYPTION_KEY` 必须是 base64 编码的 32 字节密钥，可用下面命令生成：

```bash
openssl rand -base64 32
```

## 本地启动

安装依赖：

```bash
npm install
```

启动一个本地 Postgres。任选一种即可。

Docker：

```bash
docker run --name icon-set-postgres \
  -e POSTGRES_DB=icon_set \
  -e POSTGRES_USER=icon_set \
  -e POSTGRES_PASSWORD=icon_set_password \
  -p 5432:5432 \
  -d postgres:16-alpine
```

Homebrew PostgreSQL：

```bash
brew services start postgresql@15
createdb icon_set
```

如果 `createdb icon_set` 提示数据库已存在，可以忽略。

配置后端环境变量：

```bash
cp apps/api/.env.example apps/api/.env
```

至少需要填写：

```env
APP_BASE_URL=http://localhost:5173
DATABASE_URL=postgres://icon_set:icon_set_password@127.0.0.1:5432/icon_set
ENCRYPTION_KEY=openssl-rand-base64-32-output
GITHUB_OAUTH_CLIENT_ID=your-github-oauth-client-id
GITHUB_OAUTH_CLIENT_SECRET=your-github-oauth-client-secret
CORS_ORIGIN=http://localhost:5173
COOKIE_SECURE=false
```

如果你用的是 Homebrew PostgreSQL，并且没有单独创建 `icon_set` 用户，可以把 `DATABASE_URL` 改成：

```env
DATABASE_URL=postgres://你的本机用户名@127.0.0.1:5432/icon_set
```

GitHub OAuth App 本地配置：

```txt
Homepage URL: http://localhost:5173
Authorization callback URL: http://localhost:5173/api/auth/github/callback
```

启动服务：

```bash
cargo run -p icon-set-api
npm run dev:web
```

默认前端地址是 `http://localhost:5173`，后端地址是 `http://127.0.0.1:3000`。开发时前端通过 Vite proxy 转发 `/api` 请求。

## 用户仓库配置

用户登录 `/admin` 后，在后台填写自己的 GitHub 仓库配置：

- Owner：GitHub 用户名或组织名
- Repo：用于存放图标资产的仓库名
- Branch：通常是 `main`
- Token：fine-grained personal access token

Token 权限建议：

- Repository access：只选择图标仓库
- Contents：Read and write
- Metadata：Read

## 常用命令

```bash
npm run check:web
npm run build:web
npm run check:api
```

## Docker 部署

项目提供 `docker-compose.yml`：

- `icon-set-db`：Postgres 16，持久化到 `icon-set-db-data`
- `icon-set-api`：Rust Axum API，读取 `deploy/api.env`
- `icon-set-web`：Caddy 静态前端，同时把 `/api/*` 反代到 API 容器

准备环境变量：

```bash
cp deploy/api.env.example deploy/api.env
```

编辑 `deploy/api.env`，至少设置：

```env
APP_BASE_URL=https://icon-set.example.com
ENCRYPTION_KEY=openssl-rand-base64-32-output
GITHUB_OAUTH_CLIENT_ID=your-github-oauth-client-id
GITHUB_OAUTH_CLIENT_SECRET=your-github-oauth-client-secret
CORS_ORIGIN=https://icon-set.example.com
COOKIE_SECURE=true
```

GitHub OAuth App 生产配置：

```txt
Homepage URL: https://icon-set.example.com
Authorization callback URL: https://icon-set.example.com/api/auth/github/callback
```

启动：

```bash
docker compose up -d --build
```

默认 compose 不直接暴露公网端口，适合放在 Caddy、Nginx 等反代后面。如果需要直接发布宿主机端口，可以叠加端口映射文件：

```bash
WEB_PORT=8080 docker compose -f docker-compose.yml -f docker-compose.publish.yml up -d --build
```

如果宿主机已有统一入口 Caddy，可以让外层 Caddy 加入 `icon-set` 网络，然后反代到 `icon-set-web:80`：

```bash
docker network connect icon-set your-caddy-container
```

外层 Caddyfile 示例：

```caddyfile
icon-set.example.com {
  encode zstd gzip
  reverse_proxy icon-set-web:80
}
```
