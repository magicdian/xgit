## 新增需求

### 需求:系统必须提供基于当前仓库的 push 子命令
`xgit` 必须提供 `push` 子命令，在单仓库上下文中构造并执行 `git push`，并支持显式分支参数或自动使用当前分支。

#### 场景:未显式提供分支时使用当前分支
- **当** 用户运行 `xgit push` 且未传入 `<branch>`
- **那么** 系统必须读取当前分支名作为目标分支
- **并且** 当当前分支没有可用的远端关联信息时必须返回错误，而不是静默推送到未知目标

### 需求:系统必须支持 remote 自动识别优先级
当用户未通过 `--remote` 显式指定远端时，系统必须按固定优先级识别目标 remote，并在识别失败时给出错误。

#### 场景:按优先级命中分支级配置
- **当** `branch.<branch>.pushRemote` 或 `branch.<branch>.remote` 存在
- **那么** 系统必须优先使用这些分支级配置识别 remote

#### 场景:分支级配置缺失时回退到环境与仓库配置
- **当** 分支级配置与上游 remote 都不存在
- **那么** 系统必须继续按 `XGIT_REMOTE` 与 `git config xgit.remote` 进行回退识别

#### 场景:仍无法确定时基于 remote 列表自动推断
- **当** 上述来源都无法确定 remote
- **那么** 系统必须基于 `git remote -v` 结果做自动推断（单 remote 直接使用，多 remote 按相似度与偏好项回退）
- **并且** 当仓库完全没有 remote 时必须返回错误

### 需求:系统必须支持 Gerrit 语义推送
系统必须支持通过显式参数或 remote URL 特征识别 Gerrit，并使用 `refs/for/<branch>` 作为 refspec。

#### 场景:显式启用 Gerrit
- **当** 用户传入 `--gerrit`
- **那么** 系统必须使用 `HEAD:refs/for/<branch>` 构造 refspec

#### 场景:根据 remote URL 自动识别 Gerrit
- **当** 目标 remote URL 包含 `:29418` 或 `gerrit/review/googlesource` 关键特征
- **那么** 系统必须按 Gerrit 语义构造 refspec

### 需求:系统必须转发核心 push 控制参数
`xgit push` 必须支持 `--no-thin`、`--force-with-lease` 与 `--dry-run`，并将其映射到 `git push` 参数行为。

#### 场景:dry-run 预览命令
- **当** 用户传入 `--dry-run`
- **那么** 系统必须输出将执行的 `git push` 参数预览
- **并且** 系统不得实际执行推送

#### 场景:no-thin 与 force-with-lease 传递
- **当** 用户传入 `--no-thin` 或 `--force-with-lease`
- **那么** 系统必须将对应参数传递到最终 `git push` 命令

### 需求:功能开关必须控制 push 命令可用性
当有效配置关闭 push 功能时，系统必须拒绝执行 `xgit push` 并返回可读错误信息。

#### 场景:push 功能关闭
- **当** 有效配置将 push 功能设为关闭
- **那么** 用户执行 `xgit push` 时系统必须直接报错并终止

## 移除需求
<!-- 无 -->
