## 新增需求
<!-- 无 -->

## 修改需求

### 需求:xgit reset 必须以当前分支的 upstream remote branch 为目标
`xgit reset` 必须仅在当前已切换到本地分支、且该分支存在已关联的 upstream remote branch 时执行，并将该 upstream ref 作为 reset 目标；系统必须与 `xgit push` 在同一 tracking 配置下保持一致的本地分支到 remote 分支映射解释。

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

#### 场景:与 push 共享同一映射时保持 upstream 目标一致
- **当** 用户当前位于本地分支 `8676_os6_xpdev_androidB_clean`
- **并且** 该分支 upstream 为 `origin2/8676_os6_xpdev_androidB`
- **当** 用户执行 `xgit reset`
- **那么** 系统必须将 `origin2/8676_os6_xpdev_androidB` 作为 reset 目标
- **并且** 该 upstream 分支映射必须与同条件下 `xgit push` 推导 `refs/for/8676_os6_xpdev_androidB` 的分支来源一致

## 移除需求
<!-- 无 -->
