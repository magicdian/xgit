> Imported historical task context normalized by Transpec for Trellis-first continuation. Use `.trellis/spec/` as the current source of truth; preserved source artifacts remain for provenance.

## 上下文

当前 `xgit` 已具备 `push`、`setup`、`annotate` 子命令，其中 `push` 已经实现了基于 Git 配置与 remote 列表的自动 remote 识别逻辑。相关能力主要集中在 `xgit/src/main.rs` 的命令分发和 `xgit/src/remote.rs` 的 Git 信息解析。

这次新增的两个命令都和“本地分支与 remote branch 的关系”直接相关，但它们的安全边界与 `push` 不完全相同：

- `xgit reset` 是直接改写当前分支引用的命令，必须绑定到“当前分支的真实 upstream”，不能只知道 remote 名
- `xgit checkout-remote` 是创建本地分支的命令，必须先判断 local branch 是否已存在，再判断 remote branch 是否可唯一定位

因此设计重点不是增加更多命令参数，而是把“如何确定目标 ref”与“哪些情况下必须拒绝执行”明确下来，并尽量复用现有 remote 识别代码。

## 目标 / 非目标

**目标：**

- 为 `xgit` 增加 `reset` 和 `checkout-remote` 两个子命令
- `xgit reset` 仅在当前已切到本地分支且该分支存在 upstream remote branch 时执行
- `xgit reset --hard` 明确映射到 `git reset --hard <remote>/<branch>`
- `xgit checkout-remote <remote-branch> [local-branch]` 在 remote branch 可定位、本地目标分支不存在时，映射到 `git checkout -b <local> <remote>/<remote-branch>`
- 复用并补强现有 `remote.rs`，把 upstream 解析、remote branch 存在性检查和本地分支存在性检查沉淀为可测试的 helper
- 将 `Cargo.toml` 的 `package.version` 更新为 `2604.2.1`，并保持 CLI 版本输出同步

**非目标：**

- 首版不增加 `--remote` 参数到 `checkout-remote`
- 首版不实现 `reset --soft`、`reset --mixed` 等更多 reset 模式
- 首版不把更短别名作为必须交付项，只记录推荐方向
- 不改变现有 `push` 的行为契约

## 决策

### 1. `xgit reset` 的目标必须来自当前分支的 upstream，而不是仅来自 remote 名

`reset` 需要的是完整的目标引用，例如 `origin2/8676_os6_xpdev`，而不仅仅是 `origin2`。因此实现时应新增 helper，优先读取：

- `git rev-parse --abbrev-ref --symbolic-full-name <branch>@{u}`

如果命令返回形如 `origin2/8676_os6_xpdev`，则直接将它作为 reset 目标；如果不存在 upstream，则拒绝执行。

选择这个方案的原因：

- 它天然支持“本地分支名与 remote branch 名不同”的场景
- 它比单独解析 `branch.<branch>.remote` 与 `branch.<branch>.merge` 更直接，且和 Git 的 tracking 语义一致

备选方案：

- 解析 `branch.<branch>.remote` + `branch.<branch>.merge`
  - 可行，但实现更分散，需要自行拼接 `refs/heads/*` 到 `<remote>/<branch>`
- 继续沿用 `detect_remote_for_branch`
  - 不足以得到 remote branch 名，无法满足用户示例中的非同名映射

### 2. `xgit reset` 必须限定在“当前本地分支”上下文执行

实现时需要显式区分“本地分支已 checkout”与“detached HEAD”：

- 若 `git rev-parse --abbrev-ref HEAD` 返回 `HEAD`，则视为未处于本地分支上下文，直接报错
- 不支持 `xgit reset <branch>` 这种额外位置参数，避免用户在未切换分支时误操作

备选方案：

- 允许显式传入本地分支名
  - 功能更强，但会削弱“先切分支再 reset”的安全约束，不符合本次需求

### 3. `xgit checkout-remote` 采用“优先 remote + 候选回退”的 remote 解析策略

因为命令输入不包含 remote 名，系统必须自行定位 `<remote>/<remote-branch>`。首版采用以下顺序：

1. 使用仓库级 remote 偏好作为首选 remote
   - 复用现有 remote 识别中的非分支级部分：`XGIT_REMOTE`、`git config xgit.remote`、`git remote -v` 自动推断
2. 检查首选 remote 下是否存在目标 remote branch
   - 若存在，直接使用
3. 若首选 remote 不存在该分支，则扫描所有 `refs/remotes/*/<remote-branch>`
   - 若只有一个候选，使用该候选
   - 若有多个候选，报“远端分支歧义”错误并列出候选
   - 若无候选，报“远端分支不存在”错误

这样做的原因：

- 和现有 `xgit push` 的 remote 偏好体系保持一致
- 当用户仓库里只有一个匹配 remote branch 时，仍能做到零配置使用
- 避免在多个 remote 同名分支共存时静默选错

备选方案：

- 始终扫描所有 remotes，多个候选直接失败
  - 更保守，但无法利用已有的 remote 偏好配置
- 直接复用 `detect_remote_for_branch(remote_branch_name)`
  - 对一个尚未存在的本地分支来说语义不直观，也不便表达“首选 remote 不存在该分支时回退扫描”的意图
- 为命令新增 `<remote> <branch> [local]`
  - 更明确，但超出本次用户要求

### 4. `checkout-remote` 的安全检查在执行 Git 命令前完成

在真正调用 `git checkout -b` 前，必须完成：

- 本地目标分支是否存在：检查 `refs/heads/<local-branch>`
- 远端目标分支是否存在：检查 `refs/remotes/<remote>/<remote-branch>`

只有两个条件同时满足“本地不存在 / 远端存在”时才执行命令。

这样可以把错误前置为结构化校验，而不是依赖 Git 原生命令报错后再解释。

### 5. 首版保留 `checkout-remote` 主命令名，推荐后续评估 `track` 作为别名

从语义上看，这个命令的核心动作是“从 remote branch 创建本地 tracking branch”，因此 `track` 是比 `checkout-remote` 更短也更贴近 Git 概念的候选名。

但首版仍保留 `checkout-remote`，原因是：

- 用户已明确提出该命令名
- 现阶段先落地行为约束更重要，避免在命名上引入额外讨论
- 如果后续增加别名，可以做到向后兼容

备选名称：

- `track`
- `co-remote`

推荐顺序：`track` 优于 `co-remote`，因为它更简洁且更符合“tracking branch”的行为语义。

## 风险 / 权衡

- `[风险] xgit reset --hard` 会直接丢弃工作区和索引改动
  - 缓解措施：仅在显式传入 `--hard` 时允许 hard reset，并在错误信息与帮助文案里明确其 destructive 属性

- `[风险] checkout-remote` 在多 remote 同名分支场景下可能存在目标歧义
  - 缓解措施：优先使用显式配置的首选 remote；若仍无法唯一确定则直接报错，不做静默选择

- `[风险] upstream 判断依赖 Git tracking 配置，某些本地分支可能只配置了 remote 而没有 upstream ref`
  - 缓解措施：将这类情况统一视为“没有关联 remote branch”，明确提示用户先建立 tracking 关系

- `[权衡] 首版不提供 `checkout-remote` 的显式 remote 参数`
  - 代价：极少数复杂仓库需要用户先配置首选 remote，或等待后续扩展
  - 收益：CLI 形态更简洁，符合当前需求约束

## 迁移计划

- 这是增量命令扩展，不涉及已有命令迁移
- 版本升级作为本次变更的一部分直接修改 `xgit/Cargo.toml`，目标值为 `2604.2.1`
- 实现完成后同步更新：
  - CLI help / i18n 文案
  - `README.md`
  - 对应转换前能力规范（必要时同步到 Trellis 文档）
  - 版本相关说明与 `xgit --version` 输出校验
- 若后续引入 `track` 别名，应作为兼容增强，不替换 `checkout-remote`

## 开放问题

- 是否需要在后续版本中为 `checkout-remote` 增加显式 remote 参数以处理极端多 remote 场景
- 是否需要为 `reset` 增加 `--dry-run` 或命令预览能力，以和 `push` 的体验进一步统一
