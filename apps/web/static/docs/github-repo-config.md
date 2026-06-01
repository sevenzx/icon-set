# GitHub 仓库配置指引

这份文档说明控制台「GitHub 仓库配置」应该怎么填写，以及 GitHub Token 应该怎么创建。

## 配置项说明

登录 `/console` 后，先填写 GitHub 仓库配置。保存成功后，控制台创建集合、上传图片、改名、删除等操作才会写入你的仓库。

```txt
Owner:  GitHub 用户名或组织名
Repo:   用来存放图标资产的仓库名
Branch: 写入分支，通常是 main
Token:  有目标仓库 Contents 读写权限的 GitHub Token
```

示例：

```txt
Owner: jack
Repo:  icon-set-assets
Branch: main
Token: github_pat_xxx
```

`Token` 只在保存时提交给后端。后端会用 `ENCRYPTION_KEY` 加密后存入 Postgres，后续页面不会再把 token 返回给前端。再次编辑配置时，如果不想更换 token，可以留空。

## 创建资产仓库

建议单独创建一个公开仓库，用来存放图标资源，例如：

```txt
icon-set-assets
```

仓库可以一开始是空的。第一次在控制台创建集合时，后端会自动写入：

```txt
sets.json
sets/
  {set_id}/
    manifest.json
    icons/
      {icon}.png
```

如果仓库已经存在这些文件，控制台会基于现有文件继续更新。

## 创建 GitHub Token

推荐使用 fine-grained personal access token。

进入 GitHub：

```txt
Settings -> Developer settings -> Personal access tokens -> Fine-grained tokens
```

创建 token 时建议这样配置：

```txt
Repository access: Only select repositories
Selected repository: 你的图标资产仓库
Permissions:
  Contents: Read and write
  Metadata: Read
```

其他权限不需要开启。

## 保存前的校验

保存配置时，后端会用你填写的 owner、repo、branch 和 token 访问 GitHub。

如果仓库、分支或 token 权限不正确，页面会提示错误。常见原因：

- `Owner` 或 `Repo` 拼写错误
- `Branch` 不存在，例如仓库默认分支是 `master` 但你填了 `main`
- Token 没有选择目标仓库
- Token 没有 `Contents: Read and write` 权限
- Token 过期或被撤销

`sets.json` 不存在是允许的，控制台会在第一次创建集合时生成。

## Raw URL 规则

图标上传后，manifest 里会生成 GitHub Raw URL：

```txt
https://raw.githubusercontent.com/{owner}/{repo}/refs/heads/{branch}/{path}
```

例如：

```txt
https://raw.githubusercontent.com/jack/icon-set-assets/refs/heads/main/sets/example_id/icons/logo.png
```

集合 manifest 地址：

```txt
https://raw.githubusercontent.com/{owner}/{repo}/refs/heads/{branch}/sets/{set_id}/manifest.json
```

例如：

```txt
https://raw.githubusercontent.com/jack/icon-set-assets/refs/heads/main/sets/example_id/manifest.json
```

## 数据隔离

每个登录用户都有自己的仓库配置。用户 A 保存的 owner、repo、branch、token 不会被用户 B 读取或复用。

下面这些字段会加密存储：

```txt
repo_config.github_owner
repo_config.github_repo
repo_config.github_branch
repo_config.github_token_ciphertext
```

## 建议

- 使用单独的资产仓库，不要直接写入应用源码仓库。
- Token 只授权一个资产仓库。
- Token 过期后，在 GitHub 重新生成，再回到控制台保存一次。
- 如果多人使用同一个资产仓库，建议使用组织仓库，并分别给每个人自己的 token。
