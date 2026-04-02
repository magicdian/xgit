# remote-tracking-branch-ops 规范

## 目的
待定 - 由归档变更 add-xgit-reset-and-checkout-remote 创建。归档后请更新目的。

## 需求
### 需求:xgit reset 必须以当前分支的 upstream remote branch 为目标
`xgit reset` 必须仅在当前已切换到本地分支、且该分支存在已关联的 upstream remote branch 时执行，并将该 upstream ref 作为 reset 目标。

#### 场景:当前分支跟踪不同名 remote branch 时执行普通 reset
- **当** 用户当前位于本地分支 `8676_os6_xpdev_clean`
- **并且** 该分支的 upstream 为 `origin2/8676_os6_xpdev`
- **当** 用户执行 `xgit reset`
- **那么** 系统必须展开并执行 `git reset origin2/8676_os6_xpdev`

#### 场景:显式传入 --hard 时执行 hard reset
- **当** 用户当前位于存在 upstream 的本地分支
- **当** 用户执行 `xgit reset --hard`
- **那么** 系统必须展开并执行 `git reset --hard <upstream-remote-branch>`

#### 场景:当前分支没有关联 remote branch 时拒绝执行
- **当** 用户当前位于本地分支
- **并且** 该分支不存在 upstream remote branch
- **当** 用户执行 `xgit reset`
- **那么** 系统必须拒绝执行
- **并且** 系统必须提示当前分支尚未关联 remote branch

#### 场景:detached HEAD 时拒绝执行
- **当** 当前仓库处于 detached HEAD 状态
- **当** 用户执行 `xgit reset`
- **那么** 系统必须拒绝执行
- **并且** 系统必须提示用户先切换到本地分支

### 需求:xgit checkout-remote 必须支持从 remote branch 创建本地分支
`xgit checkout-remote` 必须接收一个或两个位置参数，并在目标 remote branch 可解析且本地目标分支不存在时创建本地分支。

#### 场景:仅提供 remote branch 名时复用同名本地分支名
- **当** 用户执行 `xgit checkout-remote 8676_os6_xpdev`
- **并且** 系统可解析目标为 `origin2/8676_os6_xpdev`
- **那么** 系统必须展开并执行 `git checkout -b 8676_os6_xpdev origin2/8676_os6_xpdev`

#### 场景:显式提供 local branch 名时使用自定义本地分支名
- **当** 用户执行 `xgit checkout-remote 8676_os6_xpdev 8676_os6_xpdev_clean`
- **并且** 系统可解析目标为 `origin2/8676_os6_xpdev`
- **那么** 系统必须展开并执行 `git checkout -b 8676_os6_xpdev_clean origin2/8676_os6_xpdev`

#### 场景:本地目标分支已存在时拒绝执行
- **当** 用户执行 `xgit checkout-remote 8676_os6_xpdev`
- **并且** 本地已存在分支 `8676_os6_xpdev`
- **那么** 系统必须拒绝执行
- **并且** 系统必须提示本地分支已存在

### 需求:xgit checkout-remote 必须安全解析目标 remote
当用户只提供 remote branch 名时，系统必须按确定性规则定位目标 remote，并在无法唯一定位时拒绝执行。

#### 场景:首选 remote 存在目标分支时优先使用首选 remote
- **当** 仓库已配置首选 remote 为 `origin2`
- **并且** `refs/remotes/origin2/8676_os6_xpdev` 存在
- **当** 用户执行 `xgit checkout-remote 8676_os6_xpdev`
- **那么** 系统必须使用 `origin2/8676_os6_xpdev` 作为 checkout 来源

#### 场景:首选 remote 不命中但只有一个候选 remote branch 时回退到唯一候选
- **当** 首选 remote 不包含目标分支
- **并且** 所有 remote tracking refs 中只有一个 `<remote>/8676_os6_xpdev` 候选
- **当** 用户执行 `xgit checkout-remote 8676_os6_xpdev`
- **那么** 系统必须使用该唯一候选 remote branch

#### 场景:多个 remote 同时存在同名分支且无法唯一确定时拒绝执行
- **当** 多个 remote 都存在 `8676_os6_xpdev`
- **并且** 系统无法根据首选 remote 唯一确定目标
- **当** 用户执行 `xgit checkout-remote 8676_os6_xpdev`
- **那么** 系统必须拒绝执行
- **并且** 系统必须提示目标 remote branch 存在歧义

#### 场景:所有 remote 都不存在目标分支时拒绝执行
- **当** 所有 remote tracking refs 中都不存在目标分支
- **当** 用户执行 `xgit checkout-remote 8676_os6_xpdev`
- **那么** 系统必须拒绝执行
- **并且** 系统必须提示目标 remote branch 不存在
