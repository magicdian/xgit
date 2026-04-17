> Imported historical task context normalized by Transpec for Trellis-first continuation. Use `.trellis/spec/` as the current source of truth; preserved source artifacts remain for provenance.

## 上下文

当前 `xgit` 是一个以 `push` 为中心的轻量 CLI。参数说明和 `about` 文案直接定义在 `clap derive` 结构上，错误信息也散落在命令执行逻辑中。这种结构对单命令工具足够简单，但无法满足以下新需求：

- help 和错误提示需要在运行时切换中文/英文，并且不能把最终展示文本硬编码在代码中。
- 配置需要同时支持“用户全局默认值”和“当前 Git 项目覆盖值”，还要让不同命令共享同一套功能开关与参数。
- 设置入口需要是一个 menuconfig 风格的 ratatui UI，而不是零散的环境变量或 `git config`。
- 规范化注释不只是固定模板替换，而是要按不同变更源和不同组织规范采集运行时上下文，并针对文件类型选择渲染方式。
- 工具后续需要发布到 macOS、Linux 和 Windows，因此路径处理、home 目录定位、终端能力和外部 Git 调用都不能依赖 Unix-only 行为。

另一个现实约束是：`xgit setup --project` 必须严格依赖当前 cwd 是否位于 Git 工作区中，不能假设用户总在仓库根目录运行命令。`--latest-commit` 也会触碰已提交内容，需要在设计上明确其边界，避免默认改写历史。

## 目标 / 非目标

**目标：**

- 建立资源驱动的本地化体系，用于 CLI help、错误信息、功能状态提示和 setup TUI 文案。
- 建立统一的配置模型与分层解析规则，支持默认值、全局配置、项目配置、环境变量和 CLI 参数覆盖。
- 提供 `xgit setup` ratatui 界面，支持全局和项目作用域编辑。
- 让 `push` 和未来命令都通过统一配置层读取功能开关和参数。
- 建立规范化注释引擎，支持：
  - 运行时按策略表单采集 `reason`、引用类型和值等上下文
  - `--staged`（默认）与 `--latest-commit` 两种变更源
  - `add` / `modify` / `del` 三类模板
  - 文件类型到渲染器的映射与扩展点
- 保证核心命令在 macOS、Linux 和 Windows 上可运行，且配置、路径和 Git 调用行为具有一致性。
- 为配置解析、语言选择、功能开关、变更源识别、改动分类和渲染器建立完备的单元测试覆盖。

**非目标：**

- 第一阶段不实现 Rust 动态插件 ABI 或第三方动态加载插件机制。
- 第一阶段不要求完整实现 JSON、XML、CSV、二进制 sidecar README 等全部特殊格式渲染器。
- 第一阶段不自动执行 `git commit --amend`；`--latest-commit` 只负责准备修改结果，后续 amend 由用户显式执行。
- 第一阶段不把 `push` 的高级配置（例如 Gerrit push options）做完整实现，先提供占位与扩展槽位。
- 第一阶段不要求在单元测试之外完整打通所有平台上的端到端 UI 自动化；TUI 的跨平台验证以构建、核心状态机测试和关键手工验证为主。

## 决策

### 1. 用“bootstrap 解析 + 运行时命令树”替代纯 `clap derive`

`clap derive` 适合静态 help，但不适合根据语言包、配置值和功能开关在运行时调整命令描述。新架构将分两阶段启动：

1. 先用最小 bootstrap parser 解析 `help`、`setup --project`、版本输出等必须早期可用的参数。
2. 加载配置与语言资源后，用 `clap::Command` builder 组装完整命令树，再进入具体子命令分发。

有效语言来自项目配置、全局配置与系统语言回退链路，而不是命令行临时语言选项。这样可以在 help 中显示语言化文本，也可以在某项功能被关闭时把状态附加到命令说明上。

备选方案：

- 保留 `derive` 并双语硬编码：实现最简单，但与“代码中不能硬编码展示文案”的要求冲突。
- 在 `derive` 之外额外维护一套自定义 help：会引入两套命令定义，长期维护成本更高。

### 2. 引入统一配置解析器，并明确作用域优先级

配置来源按以下优先级合并：

1. 内置默认值
2. 全局配置 `~/.xgit/config.toml`
3. 项目配置 `<git-root>/.xgit/config.toml`
4. 环境变量覆盖
5. CLI 参数覆盖（仅用于具体功能行为，不包含语言切换）

`project` 作用域通过 `git rev-parse --show-toplevel` 定位。若命令显式要求项目作用域但当前不在 Git 工作区，则返回错误。

配置模型会包含以下稳定信息：

- `ui.lang`
- `features.push`
- `features.annotate`
- `push.*`（占位配置）
- `identity.*`
- `annotate.form.*`
- `annotate.reference_kinds.*`
- `annotate.file_rules.*`
- `annotate.policies.*`

`help` 与 `setup` 本身不受 feature toggle 关闭影响，避免把用户锁死在无法恢复的状态。

备选方案：

- 继续沿用 `git config xgit.*`：适合零散键值，但不适合表达复杂策略包和分层 UI。
- 只保留全局配置：无法满足同一用户在不同仓库使用不同规范的需求。

### 3. `xgit setup` 作为配置层的 ratatui 前端

`setup` 不只是“编辑 TOML 文件”，而是统一配置模型的交互式前端。界面采用 menuconfig 风格，至少包含以下栏目：

- UI / Language
- Features
- Push
- Identity
- Annotation Form
- Reference Kinds
- File Rules / Renderers

`xgit setup` 默认编辑全局配置，`xgit setup --project` 编辑项目配置。页头必须明确当前作用域和目标文件路径。

交互模式采用两层导航：

- 主页面：上下方向键选择栏目，Enter 进入
- 子页面：上下方向键选择字段，Enter 或编辑键修改，ESC 返回主页面

当存在未保存修改时，用户在主页面按 ESC 退出必须先经过保存确认；不得依赖 Tab 作为主页面切换的唯一方式。

备选方案：

- 用子命令逐项 set/get：可做，但对策略包、文件映射和表单定义来说可用性过差。
- 直接启动外部编辑器：缺少结构化校验，不利于发现作用域覆盖关系。

### 4. 规范化注释采用“策略包 + 运行时表单”，而不是写死 bug/req 或一上来做插件 ABI

同一仓库中的提交可能既有 bugfix 又有 feature，因此不能把 `bug` / `req` 这类值当作稳定配置。设计上将它们分成两类：

- **稳定配置**：策略包、引用类型定义、字段提示、校验规则、URL 模板、文件类型映射。
- **运行时上下文**：本次执行时填写的 `reason`、引用类型、引用 ID/URL、多引用列表等。

策略包会描述：

- `add` / `modify` / `del` 的模板
- 需要采集的运行时字段
- 引用类型列表（例如 `bug`、`req`、`custom`）
- 文件类型到渲染器的映射

这样既能适应不同公司/部门规范，也不会把“本次提交是什么类型”错误地持久化到配置文件里。

备选方案：

- 代码里写死 `bug` 和 `req`：实现快，但复用性很弱。
- 第一阶段就做动态插件：扩展性高，但对当前 CLI 的复杂度和发布成本都过高。

### 5. 注释引擎按“变更源”工作，默认 `--staged`

注释功能会显式建模变更源：

- `Staged { include_untracked: bool }`
- `LatestCommit`

默认使用 `--staged`。`staged.include_untracked` 由配置决定默认值，也可由命令参数覆盖。若开启，则把未跟踪文件视为新增文件候选。

`--latest-commit` 比较 `HEAD^..HEAD` 的变更，并将规范化结果准备到工作区/索引中，但第一阶段不自动 amend。命令会提示用户在确认结果后手动执行 `git commit --amend --no-edit`。

为避免误操作，`--latest-commit` 需要显式处理以下边界：

- 根提交没有父提交
- merge commit 的 diff 语义复杂
- 工作区存在无关脏改动

注释命令的执行链路必须从“识别改动”走到“真正落盘”，而不是停留在 preview 阶段。推荐的管线如下：

1. 读取有效配置（默认值 + 全局 + 项目覆盖）与运行时表单输入。
2. 根据 `Staged` 或 `LatestCommit` 收集目标文件的 diff hunk。
3. 以 hunk 为单位识别 `add` / `modify` / `del`。
4. 根据文件规则选择渲染器，并使用策略模板展开作者标识、日期、reason、引用类型和值。
5. 把渲染结果插入到对应文件的实际改动块附近，生成新的文件内容。
6. 按模式刷新结果：
   - `--staged`：命令成功意味着工作区文件与 index 都已经更新，`git diff --cached` 能直接看到规范化后的内容。
   - `--latest-commit`：命令成功意味着 amend 候选结果已经被物化到工作区或 index 中，用户可以直接检查并手动完成 amend。

这条链路要求实现明确区分 preview 与 apply。第一阶段可以保留调试预览能力，但正式命令成功语义必须是“结果已被写入目标内容”，不能是“仅打印渲染结果”。

在 `--staged` 模式下，如果同一目标文件存在会破坏 staged 语义的未暂存改动，实现应优先选择安全失败并提示用户，而不是静默覆盖或宣称规范化完成。

### 6. 渲染器抽象先落 C-like，特殊格式只保留扩展点

文件规则会把文件类型映射到渲染器，例如：

- `c_line_block`
- `xml_comment`
- `json_key_comment`
- `sidecar_readme`

第一阶段只要求完整实现 `c_line_block`，满足 `.h`、`.c`、`.cpp`、`.java` 等 C-like 源文件。其他渲染器先以配置模型和扩展点的形式存在，未实现时要能明确提示或跳过，而不是 silently fail。

渲染器本身不得把最终输出写死为内部调试样式。最终注释文本必须由 `annotate.policies.add`、`annotate.policies.modify`、`annotate.policies.del` 模板驱动，并结合 `identity.author_tag` 等身份字段与运行时输入展开。至少需要稳定支持以下占位信息：

- 作者标识（例如姓名缩写）
- 当前日期
- `reason`
- 引用类型与引用值
- `old` 代码内容（适用于 `modify` / `del`）
- 实际新增或修改后的代码块

这样才能支持同一套注释引擎在不同团队模板之间切换，而不是把当前公司的格式写死在渲染器实现里。

### 7. 跨平台兼容性作为一等约束建模

实现必须避免把 Unix shell、路径分隔符或终端行为假设写死在命令层。具体约束包括：

- 使用 `std::path::PathBuf`、`dirs`/`home` 等跨平台 API 解析 home 目录和配置路径。
- 调用 Git 时直接使用 `std::process::Command`，不通过 shell 拼接命令字符串执行。
- 配置目录、项目目录和文件规则匹配需要兼容 Windows 路径分隔符。
- TUI 启动前必须检查终端能力，并为不支持的环境提供明确失败提示。

备选方案：

- 先以 macOS/Linux 为主，Windows 后补：短期更快，但会把路径与终端假设沉淀到代码里，后续补 Windows 成本更高。
- 用平台特定分支分别实现：可行，但在当前项目规模下会增加维护负担，应优先使用平台无关抽象。

### 8. 测试策略以“单元测试优先 + 平台矩阵验证”为主

测试策略分三层：

1. **单元测试**：覆盖配置合并、语言选择、功能开关、Git 工作区定位、diff 分类、策略渲染、命令构建、设置界面状态机与退出确认等纯逻辑模块。
2. **平台兼容测试**：至少保证项目在 macOS、Linux 和 Windows 上能够编译并运行核心测试集。
3. **手工验证**：对 TUI 和 Git 真实交互做关键路径确认，补足终端能力和平台差异的不可完全单元化部分。

这样可以在保持实现节奏的同时，把“完备单元测试”作为硬门槛而不是收尾补丁。

## 风险 / 权衡

- 运行时命令树替换静态 `clap derive` 会提高启动流程复杂度 → 用最小 bootstrap parser 限制早期分支，并为帮助输出编写快照测试。
- 分层配置可能让用户不清楚某个值来自哪里 → setup UI 中显示作用域与目标文件路径，并在后续迭代考虑增加“值来源”提示。
- setup 若出现“菜单中文、内容英文”的混合语言，会显著降低可用性 → 所有 setup 可见文本统一走语言资源，并为界面文案增加回归测试。
- staged 模式如果直接写工作区文件并重新 stage，可能影响用户未暂存修改 → 第一阶段只处理目标文件，并在实现时明确区分 index 与 worktree 更新路径。
- latest-commit 如果自动 amend 风险较高 → 第一阶段明确不自动 amend，由用户手动完成历史改写。
- 策略包比写死模板更灵活，但配置模型会更大 → 先限制首批字段和渲染器，保证 MVP 可交付。
- Windows 终端、路径与文件锁语义可能与 Unix 平台存在差异 → 通过平台无关 API、平台矩阵构建和关键兼容性测试提前发现问题。
- “完备单元测试”会增加前期工作量 → 通过把逻辑拆成可测试模块，降低后续回归成本并提高跨平台迭代稳定性。

## Migration Plan

1. 引入默认配置与语言资源后，未创建任何配置文件的用户仍然可以继续使用现有 `xgit push` 默认行为。
2. 新增 `setup` 后，只有在用户显式保存时才创建 `~/.xgit/config.toml` 或 `<git-root>/.xgit/config.toml`。
3. `push` 功能接入新配置层时，保持旧行为为默认值，避免在没有配置文件的仓库里出现行为回归。
4. 注释规范化功能以新子命令落地，不改变现有 push 工作流；需要时可以在后续增量里接入 hook 或更自动化的流程。
5. 在进入实现阶段时，为 macOS、Linux 和 Windows 建立构建/测试验证步骤，确保新增模块不会在单一平台上才可用。

## Open Questions

- `identity.author_tag` 的默认值应优先来自 setup 显式配置，还是允许回退到 `git config user.name` / `user.email` 派生？
- 第一阶段是否需要内置多引用支持，还是只要求单引用并把多引用留到下一轮？
- `--latest-commit` 在存在工作区脏改动时，是强制拒绝，还是允许通过额外确认继续？
- 特殊文件渲染器的第一批落地范围是否只做 `xml_comment`，还是全部后置到下一条变更？
- Windows 控制台环境下，ratatui 的降级策略是否只提示不支持，还是需要提供非交互 fallback？
