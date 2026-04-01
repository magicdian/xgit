# 设计：xgit push（远端识别与 push 构造）

## 总览
`xgit` 为单仓库工具，`push` 子命令负责：
1. 确定目标分支（参数或当前分支）
2. 识别目标 remote（按优先级规则）
3. 判定是否使用 Gerrit 语义（refs/for）
4. 构造并执行 `git push`（支持 `--no-thin`、`--force*` 等）

实现原则：调用系统 `git`（`std::process::Command`）以保持与用户环境（credential helper、SSH、hooks）一致。

## CLI 参数（推荐）
- `xgit push [<branch>]`：推送当前 HEAD 到目标分支（若省略 branch，则使用当前分支名）
- `--remote <name>`：强制使用指定 remote
- `--gerrit`：强制使用 Gerrit 语义（push 到 `refs/for/<branch>`）
- `--no-thin`：将 `--no-thin` 传递给 `git push`
- `--force`, `--force-with-lease`：转发强制推送参数
- `--dry-run`：仅打印要执行的命令
- `--verbose`：打印检测细节

## 远端识别算法（单仓库，优先级）
1. 分支相关配置（最高优先级）
   - `git config --get branch.<branch>.pushRemote`
   - `git config --get branch.<branch>.remote`
   - `git rev-parse --abbrev-ref --symbolic-full-name @{u}` → 若有上游，解析 remote 名称
2. 环境/显式配置
   - ENV `XGIT_REMOTE` 或 `git config xgit.remote`
3. 自动推断
   - `git remote -v`：若仅有一个 remote，则选它
   - 若多个：比较每个 remote 的 URL 路径与本地 origin（或仓库路径）相似度（最后 2-3 段）；若多个 URL 实质相同，按优先名单选择（默认 `origin`, `origin2`, `upstream`）
4. 决策失败
   - 交互模式：列候选让用户选择
   - 非交互/CI：返回错误并提示 `--remote`

注：实现时将把“优先名单”作为可配置项（`git config xgit.preferredRemotes` 或用户 config 文件）。

## Gerrit 检测规则
- URL 含端口 `:29418` 或 hostname/path 包含关键字 `gerrit`、`review`、`googlesource` → 认为是 Gerrit
- 或命令行 `--gerrit` 强制指定
- Gerrit 时构造的 ref：`refs/for/<branch>`（支持后续扩展的 push-option）

## 构造命令示例
- 普通：`git push <remote> HEAD:<branch>`
- Gerrit：`git push <remote> HEAD:refs/for/<branch>`
- 含 no-thin：`git push --no-thin <remote> HEAD:...`
- 含 force：`git push --force-with-lease <remote> ...`

CLI -> 内部行为伪码：
```
branch = arg_branch or get_current_branch()
remote = if opt_remote { opt_remote } else { detect_remote(branch) }
is_gerrit = opt_gerrit or detect_gerrit(remote)
refspec = is_gerrit ? "HEAD:refs/for/" + branch : "HEAD:" + branch
cmd = ["git", "push"]
if no_thin { cmd.push("--no-thin") }
if force_with_lease { cmd.push("--force-with-lease") }
cmd.push(remote)
cmd.push(refspec)
run_or_print(cmd, dry_run)
```

## 错误处理与退出码
- 找不到 remote：退出码 2，输出可操作提示（`--remote`）
- git push 返回非零：将原样返回并附带简短建议
- 非交互模式下的歧义：退出码 3，并列出候选 remotes

## 配置与扩展点
- 支持 `git config xgit.remote`、`git config xgit.preferredRemotes` 与 ENV `XGIT_REMOTE`
- 后续支持：Gerrit push-options、topic、批量模式、VSCode 插件

## 测试计划
- 单元测试：远端识别逻辑（模拟 `git remote -v` 输出）
- 集成测试：在临时仓库中创建两个 remotes（同/不同 URL），验证 `--dry-run` 下构造命令
- 实际环境验证：在含 Gerrit remote 的真实仓库中运行 `--dry-run` 与真实 push（手动）

## 安全与注意事项
- `--no-thin` 会增大上传体积，网络慢时谨慎使用
- `--force` 系列需谨慎，建议默认不启用
