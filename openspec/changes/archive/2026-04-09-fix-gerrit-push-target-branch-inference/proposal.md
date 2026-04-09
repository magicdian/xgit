## 为什么

当前 `xgit push` 在 Gerrit 语义下直接使用本地分支名构造 `refs/for/<branch>`。当本地分支名与 upstream 远端分支名不一致（例如本地 `8676_os6_xpdev_androidB_clean` 跟踪 `origin2/8676_os6_xpdev_androidB`）时，会推送到错误目标并被 Gerrit 拒绝。

同时，`xgit reset` 虽然目标上应使用 upstream，但当前缺少与 `push` 一致的统一分支映射入口，存在后续命令行为分叉和回归风险。

该问题会直接阻断提交流程并带来一致性隐患，需要修复 `push` 分支名解析、校验 `reset` 场景，并统一本地分支到 remote 分支映射入口；同时同步更新版本号以发布修复。

## 变更内容

- 修正 `xgit push` 在 Gerrit 场景下的目标分支推导逻辑：
- 当未显式传入 `<branch>` 且当前分支存在 upstream 时，`refs/for/<branch>` 中的 `<branch>` 必须使用 upstream 的远端分支名，而不是本地分支名。
- 校验 `xgit reset` 在“本地分支名与 upstream 分支名不同”场景下的行为，确保始终以 upstream remote branch 为目标。
- 在 `remote` 模块抽取统一的“本地分支到 remote 分支映射”入口，供 `push` 与 `reset` 复用，避免命令间分支映射规则漂移。
- 保持现有兼容行为：显式传入 `<branch>` 时仍按用户输入构造 refspec；不存在 upstream 时仍按现有校验报错。
- 增加/调整测试，覆盖 push 与 reset 在异名分支映射场景下的一致性结果。
- 将项目版本更新为 `2604.4.1`。

## 功能 (Capabilities)

### 新增功能
- 无

### 修改功能
- `remote-aware-push`: 调整 Gerrit refspec 分支来源规则，确保 `xgit push` 在 upstream 异名分支场景下推送到正确 `refs/for/*` 目标。
- `remote-tracking-branch-ops`: 明确 `xgit reset` 与 `xgit push` 在本地/远端分支映射上的一致性约束，避免命令间映射分叉。
- `versioning-source-unification`: 将本次规范中的版本目标值更新为 `2604.4.1`。

## 影响

- 受影响代码：`xgit/src/main.rs`（push/reset 分支解析与命令构造）、`xgit/src/remote.rs`（统一映射入口与 upstream 分支信息提取/复用）、相关单元测试。
- 受影响配置与发布：`xgit/Cargo.toml` 的 `package.version`。
- 受影响行为：Gerrit 推送目标分支解析、`xgit push --dry-run` 输出内容、`xgit reset` 的 upstream 目标一致性。
