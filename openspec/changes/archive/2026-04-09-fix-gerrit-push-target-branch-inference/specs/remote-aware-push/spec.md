## 新增需求
<!-- 无 -->

## 修改需求

### 需求:系统必须提供基于当前仓库的 push 子命令
`xgit` 必须提供 `push` 子命令，在单仓库上下文中构造并执行 `git push`，并支持显式分支参数或自动使用当前分支；当未显式传入 `<branch>` 时，系统必须以当前分支作为 remote 识别基础，并在 Gerrit 语义下优先使用 upstream 远端分支名构造 `refs/for/<branch>` 目标。

#### 场景:未显式提供分支时使用当前分支进行 remote 识别
- **当** 用户运行 `xgit push` 且未传入 `<branch>`
- **那么** 系统必须读取当前分支名作为 remote 识别所用分支
- **并且** 当当前分支没有可用的远端关联信息时必须返回错误，而不是静默推送到未知目标

#### 场景:未显式提供分支且 upstream 分支名不同于本地分支名
- **当** 用户当前位于本地分支 `8676_os6_xpdev_androidB_clean`
- **并且** 当前分支 upstream 为 `origin2/8676_os6_xpdev_androidB`
- **并且** 目标 remote 被识别为 Gerrit remote
- **当** 用户执行 `xgit push`
- **那么** 系统必须构造 `HEAD:refs/for/8676_os6_xpdev_androidB`
- **并且** 系统禁止构造 `HEAD:refs/for/8676_os6_xpdev_androidB_clean`

### 需求:系统必须支持 Gerrit 语义推送
系统必须支持通过显式参数或 remote URL 特征识别 Gerrit，并使用 `refs/for/<branch>` 作为 refspec；当未显式传入 `<branch>` 且存在 upstream 时，`<branch>` 必须优先取 upstream 远端分支名。

#### 场景:显式启用 Gerrit
- **当** 用户传入 `--gerrit`
- **那么** 系统必须使用 `HEAD:refs/for/<branch>` 构造 refspec

#### 场景:根据 remote URL 自动识别 Gerrit
- **当** 目标 remote URL 包含 `:29418` 或 `gerrit/review/googlesource` 关键特征
- **那么** 系统必须按 Gerrit 语义构造 refspec

#### 场景:显式传入 branch 时保持用户指定目标
- **当** 用户执行 `xgit push 8676_os6_xpdev_androidB`
- **并且** 目标 remote 被识别为 Gerrit remote
- **那么** 系统必须构造 `HEAD:refs/for/8676_os6_xpdev_androidB`
- **并且** 系统不得用当前分支 upstream 覆盖用户显式传入的分支参数

## 移除需求
<!-- 无 -->
