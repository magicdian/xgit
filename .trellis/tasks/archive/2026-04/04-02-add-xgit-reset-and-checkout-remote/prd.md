> Imported historical task context normalized by Transpec for Trellis-first continuation. Use `.trellis/spec/` as the current source of truth; preserved source artifacts remain for provenance.

## 为什么

当前仓库里已经有 `xgit push` 这种基于分支 remote 关系做自动展开的命令，但开发者在日常同步分支时仍要手写 `git reset <remote>/<branch>` 或 `git checkout -b <local> <remote>/<branch>`。这些操作在多 remote、分支名不完全一致的场景里容易输错，也缺少统一的前置校验。

现在补齐 `xgit reset` 和 `xgit checkout-remote`，可以把“当前分支关联哪个 remote branch”“是否允许覆盖本地状态”“创建本地分支前是否存在同名分支”这些规则固化到工具里，减少误操作并保持和现有 `xgit push` 一致的使用体验。

## 变更内容

- 新增 `xgit reset [--hard]`
- `xgit reset` 仅允许在“当前已切到本地分支，且该分支存在已关联的 remote branch”时执行；否则直接报错并终止
- `xgit reset` 默认展开为 `git reset <remote>/<remote-branch>`，`xgit reset --hard` 展开为 `git reset --hard <remote>/<remote-branch>`
- 新增 `xgit checkout-remote <remote-branch-name> [local-branch-name]`
- 当 `xgit checkout-remote` 只提供一个参数时，系统必须将其同时作为 remote branch 名和 local branch 名
- `xgit checkout-remote` 在执行前必须检查本地是否已存在目标 local branch；若已存在则拒绝执行
- `xgit checkout-remote` 必须基于当前仓库可识别的 remote 生成 `git checkout -b <local> <remote>/<remote-branch>` 命令
- 将项目版本号更新到 `2604.2.1`
- 设计阶段将评估更短的命令名建议；本次变更先保留 `checkout-remote` 作为首版命令名，并给出推荐别名方向，不把别名作为首版强制范围

## 功能 (Capabilities)

### 新增功能
- `remote-tracking-branch-ops`: 提供基于 remote tracking 关系的分支重置与远端分支检出能力

### 修改功能
- `versioning-source-unification`: 将本次落地后的 `package.version` 目标值更新为 `2604.2.1`

## 影响

- 受影响代码：
  - `xgit/Cargo.toml`
  - `xgit/src/main.rs`
  - `xgit/src/remote.rs`
  - `xgit/resources/i18n/zh-CN.toml`
  - `xgit/resources/i18n/en-US.toml`
  - `xgit/README.md`
- 可能新增针对 Git 分支/上游关系解析的辅助函数与测试
- 需要新增或扩展转换后的 Trellis 规范文档以覆盖 reset 与 checkout-remote 的行为约束
