# GitHub 仓库配置指引

Icon Set 会把你上传的图标、集合清单和 `manifest.json` 写入 GitHub 仓库。第一次进入控制台时，需要先告诉系统：写到哪个仓库、哪个分支，以及用哪个 GitHub Token 授权写入。

## 第一步：准备一个图标资产仓库

建议单独创建一个仓库来存放图标资产，例如：

```txt
icon-set-assets
```

这个仓库可以是空的。保存配置后，你在控制台创建集合、上传图标、改名或删除时，Icon Set 会自动写入需要的文件：

```txt
sets.json
sets/
  {set_id}/
    manifest.json
    icons/
      {icon}.png
```

仓库已经有这些文件也没关系，控制台会基于现有内容继续更新。

## 第二步：创建 GitHub Token

推荐使用 GitHub 的 fine-grained personal access token，只授权这个图标资产仓库。

在 GitHub 里进入：

```txt
Settings -> Developer settings -> Personal access tokens -> Fine-grained tokens
```

创建 token 时建议这样选择：

```txt
Repository access: Only select repositories
Selected repository: 你的图标资产仓库
Permissions:
  Contents: Read and write
  Metadata: Read
```

其他权限不需要开启。

## 第三步：填写控制台配置

回到 `/console`，填写「GitHub 仓库配置」：

```txt
Owner:  GitHub 用户名或组织名
Repo:   图标资产仓库名
Branch: 写入分支，通常是 main
Token:  刚创建的 GitHub Token
```

示例：

```txt
Owner: jack
Repo: icon-set-assets
Branch: main
Token: github_pat_xxx
```

保存成功后，控制台的创建集合、上传图片、改名、删除等操作都会写入这个仓库。

Token 只需要在首次配置或更换 Token 时填写。再次编辑 owner、repo、branch 时，如果不想更换 Token，可以把 Token 输入框留空。

## Token 如何保存

Token 只会在保存配置时提交一次。后端会加密存储 Token，后续页面不会再把 Token 明文返回给前端。

再次打开配置页时，Token 输入框会保持为空。这是正常表现，表示当前配置已经有已保存的 Token。如果不想更换 Token，直接留空保存即可；只有重新填写 Token 时，系统才会更新它。

## 常见保存失败原因

保存时，系统会尝试访问你填写的仓库和分支。如果失败，通常是下面几类问题：

- `Owner` 或 `Repo` 拼写不正确。
- `Branch` 不存在，例如仓库默认分支是 `master`，但你填写了 `main`。
- Token 没有选择这个仓库。
- Token 没有 `Contents: Read and write` 权限。
- Token 已经过期、被撤销，或者复制时少了一段。

如果仓库里还没有 `sets.json`，这是正常的。第一次创建集合时，Icon Set 会自动生成。

## 如何使用生成的地址

图标上传后，集合 manifest 里会保存 GitHub Raw 图片地址。你可以在页面里复制这些地址，用在前端项目、Markdown、文档或其他工具中。

```txt
https://raw.githubusercontent.com/{owner}/{repo}/refs/heads/{branch}/{path}
```

例如：

```txt
https://raw.githubusercontent.com/jack/icon-set-assets/refs/heads/main/sets/example_id/icons/logo.png
```

每个集合也有自己的 manifest 地址：

```txt
https://raw.githubusercontent.com/{owner}/{repo}/refs/heads/{branch}/sets/{set_id}/manifest.json
```

例如：

```txt
https://raw.githubusercontent.com/jack/icon-set-assets/refs/heads/main/sets/example_id/manifest.json
```

## 使用建议

- 单独准备一个图标资产仓库，不要直接写入应用源码仓库。
- Token 只授权这个资产仓库，权限保持最小。
- Token 过期后，在 GitHub 重新生成，再回到控制台保存一次。
