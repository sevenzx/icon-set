# Icon Set

一个使用 `SvelteKit + Axum` 构建的图标集合管理应用。公开页面读取图标集合，管理员后台通过 Rust 后端安全写入 GitHub 公开仓库。

## 功能

- 按 set 管理不同图标库，例如 `emby`、`sub`
- 图标图片和 manifest 存放在公开 GitHub 仓库
- 前台浏览图标集合、搜索图标、复制图标 raw URL 和集合 manifest raw URL
- 公开页面不展示后台入口，管理员直接访问 `/admin`
- 管理后台支持创建 set、编辑集合信息、上传图片、修改图标 name、删除图标
- 管理后台支持多选图片或 zip 压缩包批量上传，批量图片总体积限制为 10MB
- 上传图片时会在当前集合内校验图片内容 MD5，避免同一集合重复收录相同图片
- 管理后台写接口同时校验 session cookie 和登录后签发的 `X-Admin-Token`，避免重复传递后台密码
- API 为后台操作提供登录限流、同源 CSRF 校验、短时防重复提交和审计日志
- 删除集合和删除图标使用二段式确认弹窗，需要输入目标 ID 或名称后才会执行
- GitHub Token 只存在 Axum 后端环境变量中，前端不会接触写权限

## 仓库数据结构

后端会在你配置的公开 GitHub 仓库里维护下面的结构：

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
      "url": "https://raw.githubusercontent.com/owner/repo/main/sets/emby/icons/emby.png"
    }
  ],
  "updated_at": "2026-05-29T00:00:00Z"
}
```

## 本地启动

安装前端依赖：

```bash
npm install
```

配置后端环境变量：

```bash
cp apps/api/.env.example apps/api/.env
```

然后编辑 `apps/api/.env`：

```env
ADMIN_PASSWORD=你的后台密码
GITHUB_OWNER=你的 GitHub 用户或组织
GITHUB_REPO=公开图标仓库名
GITHUB_BRANCH=main
GITHUB_TOKEN=有 Contents 读写权限的 token
```

启动后端：

```bash
cargo run -p icon-set-api
```

启动前端：

```bash
npm run dev:web
```

默认前端地址是 `http://localhost:5173`，后端地址是 `http://127.0.0.1:3000`。开发时前端通过 Vite proxy 转发 `/api` 请求。

前端复制集合地址时会生成 `sets/{id}/manifest.json` 的 raw GitHub URL。默认资源根地址是：

```env
VITE_RAW_ASSET_BASE_URL=https://raw.githubusercontent.com/sevenzx/icon-set-assets/refs/heads/main
```

如果部署到其他公开资源仓库，在 `apps/web/.env` 中覆盖这个值。

## GitHub Token 权限

公开仓库建议使用 fine-grained personal access token：

- Repository access：只选择图标仓库
- Contents：Read and write
- Metadata：Read

不要把 `GITHUB_TOKEN` 放到前端 `.env`，也不要提交 `.env` 文件。

## 常用命令

```bash
npm run check:web
npm run build:web
npm run check:api
```

## Docker 部署

项目提供 `docker-compose.yml`：

- `icon-set-api`：Rust Axum API，读取 `deploy/api.env`
- `icon-set-web`：Caddy 静态前端，同时把 `/api/*` 反代到 API 容器

准备部署环境变量：

```bash
cp deploy/api.env.example deploy/api.env
```

然后编辑 `deploy/api.env`，至少设置 `ADMIN_PASSWORD`、`GITHUB_OWNER`、`GITHUB_REPO`、`GITHUB_BRANCH`、`GITHUB_TOKEN`、`CORS_ORIGIN`。容器内 API 监听地址会由 compose 覆盖为 `0.0.0.0:3000`。

启动：

```bash
docker compose up -d --build
```

默认 compose 不直接暴露公网端口，适合放在 Caddy、Nginx 等反代后面。如果需要直接发布宿主机端口，可以叠加端口映射文件：

```bash
WEB_PORT=8080 docker compose -f docker-compose.yml -f docker-compose.publish.yml up -d --build
```

如果宿主机已有一个统一入口 Caddy，可以让外层 Caddy 加入 `icon-set` 网络，然后反代到 `icon-set-web:80`：

```bash
docker network connect icon-set your-caddy-container
```

外层 Caddyfile 示例：

```caddyfile
icon-set.19970925.xyz {
  encode zstd gzip
  reverse_proxy icon-set-web:80
}
```

同域部署时，`deploy/api.env` 建议设置：

```env
CORS_ORIGIN=https://icon-set.19970925.xyz
COOKIE_SECURE=true
```

## 部署建议

前端可以静态部署到任意平台，例如 Vercel、Netlify、Cloudflare Pages。后端需要一个能运行 Rust 服务的环境，例如 Fly.io、Render、Railway、VPS 或容器平台。

生产环境如果前后端不同域名，需要设置：

```env
CORS_ORIGIN=https://你的前端域名
COOKIE_SECURE=true
```

如果前后端同域反代，前端可继续使用相对路径 `/api`。
