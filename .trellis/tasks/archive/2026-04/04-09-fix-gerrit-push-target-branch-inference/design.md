> Imported historical task context normalized by Transpec for Trellis-first continuation. Use `.trellis/spec/` as the current source of truth; preserved source artifacts remain for provenance.

## 上下文

当前 `xgit push` 在 `xgit/src/main.rs` 中将同一个 `branch` 变量同时用于两件事：

- remote 识别（`detect_remote_for_branch(&branch)`）
- refspec 构造（Gerrit 时 `HEAD:refs/for/{branch}`）

该实现默认“本地分支名就是目标远端分支名”。在用户常见的清理分支命名场景（例如本地 `*_clean` 跟踪远端正式分支）下，Gerrit 需要的目标应是 upstream 分支名，而不是本地分支名，导致当前行为会被 Gerrit 拒绝。

`xgit reset` 已经走 upstream 目标路径，但目前 `push` 与 `reset` 的分支映射逻辑入口分散在调用侧，缺少统一映射抽象，后续容易出现“一个命令修了、另一个命令漂移”的维护问题。

## 目标 / 非目标

**目标：**

- 修复 `xgit push` 未显式传入 `<branch>` 时的 Gerrit refspec 分支推导：优先使用当前分支 upstream 的远端分支名。
- 检查并固化 `xgit reset` 在异名分支映射场景的行为，确保与 `push` 使用一致的映射来源。
- 在 `remote` 层提供统一的本地分支到 remote 分支映射入口，避免 push/reset 逻辑分叉。
- 保持 remote 自动识别优先级和现有 CLI 参数行为不变。
- 为“本地分支名与 upstream 分支名不一致”场景补充可回归测试（覆盖 push 与 reset）。
- 将 `xgit/Cargo.toml` 的 `package.version` 更新为 `2604.4.1`。

**非目标：**

- 不改变 `xgit push <branch>`（显式传入分支）的行为语义。
- 不在本次引入新的 `push` 参数或改变 Gerrit 自动识别规则。
- 不改变 `reset` 与 `push` 的用户可见命令参数形态。
- 不引入新的 remote 推断策略，仅统一已有 upstream 映射入口。

## 决策

### 1. 区分“remote 识别分支”与“Gerrit 目标分支”

`execute_push` 中新增两个语义值：

- `selection_branch`：用于 remote 识别，保持现有逻辑（显式 `<branch>` 用该值；未显式时用当前本地分支）。
- `target_branch`：用于 refspec 构造；在 Gerrit 且未显式 `<branch>` 时，从 upstream 中解析远端分支名，否则回退到 `selection_branch`。

原因：

- 避免把“选 remote 的依据”和“refs/for 的目标”绑定为同一字符串。
- 兼容现有 remote 识别代码与分支级配置读取逻辑。

备选方案：

- 始终用 upstream 分支名替换 `branch`：
  - 风险是改变显式 `xgit push <branch>` 的预期，且可能影响 `branch.<name>.*` 配置读取。
- 仅靠字符串裁剪（如移除 `_clean` 后缀）推断目标分支：
  - 不稳定且不可泛化，容易在命名约定变化时失效。

### 2. upstream 解析使用 Git tracking 真值，不引入额外约定

继续基于 `git rev-parse --abbrev-ref --symbolic-full-name <branch>@{u}` 获取 upstream，如返回 `origin2/8676_os6_xpdev_androidB`，则提取分支部分 `8676_os6_xpdev_androidB` 作为 Gerrit 目标分支。

原因：

- 与 Git 原生 tracking 语义一致，天然支持本地/远端异名分支。
- 与现有 `remote::get_upstream_remote_branch` 实现一致，可在其上增加“提取 branch 名”的轻量 helper。

备选方案：

- 解析 `branch.<branch>.merge` 再自行转换：
  - 可行但实现更分散，边界条件更多。

### 3. 建立 push/reset 共用的分支映射入口

在 `remote.rs` 增加统一 helper（例如返回 `remote`、`remote_branch`、`full_upstream_ref` 的结构），由 `push` 与 `reset` 复用：

- `push`：在 Gerrit 且未显式 `<branch>` 时，读取统一映射入口中的 `remote_branch` 作为 `refs/for/*` 目标。
- `reset`：继续使用 `full_upstream_ref` 作为 reset 目标，但由同一映射入口提供数据来源。

原因：

- 把“本地分支 -> upstream 映射”收敛为单一真值来源。
- 后续若扩展 `checkout-remote` 或其他分支命令，可直接复用，减少重复解析。

备选方案：

- 仅在 `push` 内局部修复，不触碰 `reset`：
  - 短期改动小，但会保留两个命令各自解析 upstream 的重复逻辑。

### 4. 测试策略采用“纯函数优先 + 关键路径覆盖”

- 在 `remote.rs` 增加 upstream 分支提取 helper 的单元测试（包含多级路径分支名）。
- 在 push 相关路径增加最小可测单元，验证 Gerrit refspec 在“未显式 branch + upstream 异名”场景下展开为 `HEAD:refs/for/<upstream-branch>`。
- 在 reset 相关路径增加一致性测试，验证其目标引用来自同一 upstream 映射来源并保持 `git reset <remote>/<branch>` 结果不变。
- 保留既有 `--dry-run` 行为，以便人工验证命令预览输出。

原因：

- 减少对真实 Git 仓库环境的耦合，测试稳定性更高。
- 能直接锁定本次回归点，防止后续重构再引入同类问题。

## 风险 / 权衡

- `[风险]` 分支推导路径变复杂，若变量语义混用仍可能回归  
  缓解措施：明确 `selection_branch` 与 `target_branch` 命名，并补充对应测试。

- `[风险]` 某些仓库 tracking 配置异常，upstream 读取失败  
  缓解措施：统一映射入口保留“无 upstream 则报错/返回空”的既有语义；`push` 与 `reset` 在各自命令上下文中复用同一结果，不做静默兜底。

- `[权衡]` 本次不引入显式 `--target-branch` 参数  
  收益：保持 CLI 简洁并最小化变更范围；代价：高级场景仍依赖 Git tracking 配置正确性。

## 迁移计划

- 代码迁移：在 `remote` 模块新增统一分支映射入口，并调整 `execute_push` 与 `execute_reset` 复用该入口；补充 helper 与测试。
- 版本迁移：将 `xgit/Cargo.toml` 的 `package.version` 更新为 `2604.4.1`。
- 验证步骤：
- 单元测试通过（尤其是 push/reset 分支映射一致性相关）。
- 在真实仓库执行 `xgit push --dry-run`，确认输出从 `refs/for/<local-branch>` 变为 `refs/for/<upstream-branch>`。
- 在真实仓库执行 `xgit reset`（或对应 dry-run/预览验证流程）确认目标仍为 upstream full ref，且与统一映射入口结果一致。
- 回滚策略：若发现兼容问题，回退到上一版本并撤销本次分支推导改动（不影响仓库数据，仅影响命令构造）。

## 开放问题

- 是否需要在后续版本支持显式 `--target-branch`，用于覆盖 tracking 配置缺失或特殊发布流程。
- 是否应在错误提示中增加“检测到本地与 upstream 分支名不一致”的专项文案，以降低排障成本。
