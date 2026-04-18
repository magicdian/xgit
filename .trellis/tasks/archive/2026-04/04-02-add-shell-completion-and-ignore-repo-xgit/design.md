> Imported historical task context normalized by Transpec for Trellis-first continuation. Use `.trellis/spec/` as the current source of truth; preserved source artifacts remain for provenance.

## 上下文

当前 `xgit` 的 CLI 入口集中在 [xgit/src/main.rs](../../../../../xgit/src/main.rs) 的 `build_runtime_command` 中，命令树由 `clap` 在运行时结合有效语言与功能开关构建。这个结构已经天然具备“单一命令模型”的条件，适合把 help、参数解析和 shell completion 统一到同一份定义上。

另一方面，`annotate` 命令在不同路径上都会接触文件候选集合：默认 staged 模式会处理暂存改动，并可按配置纳入 untracked 文件；`--latest-commit` 模式会先做工作区洁净校验后再读取最后一次提交变更。仓库级配置目录 `<git-root>/.xgit/` 恰好既是项目配置来源，又常常不会提交到仓库，因此它现在会在这些路径中被误判为普通仓库文件，导致 `annotate` 行为与“仓库配置优先级”相冲突。现有的项目配置优先级已经由 `layered-configuration` 规范定义，本次设计不能破坏这条约束。

## 目标 / 非目标

**目标：**

- 为 `xgit` 增加一个标准的 shell completion 输出入口，至少覆盖 macOS 和 Linux 常见的 `zsh`、`bash`、`fish`
- 在 `xgit completion --install` 中自动识别当前终端 shell，并通过“先预览、后确认”的方式执行安装：先写入临时目录，再提示将写入的补全文件与配置文件，只有用户输入 `Y`/`y` 才继续
- 补全脚本必须复用当前 `clap` 命令树，确保新增命令和参数后不会出现 help 与 completion 漂移
- 首版补全聚焦命令名、子命令名和静态参数名，优先解决发现性问题
- 将项目版本号更新到 `2604.2.2`
- 调整 `annotate` 命令的候选筛选与前置校验，让仓库根目录下的 `.xgit/` 及其内容不再进入格式化候选，也不再阻塞执行
- 保持 `<git-root>/.xgit/config.toml` 继续作为项目级高优先级配置来源

**非目标：**

- 首版不实现基于当前 Git 仓库实时解析的动态参数值补全，例如 remote branch、当前分支名或引用类型候选
- 不改变现有配置层级合并逻辑
- 不移除 `latest-commit` 模式对“真实源码改动必须干净”的安全约束
- 不把任意其他未跟踪目录加入忽略名单

## 决策

### 1. 使用新的 `xgit completion <shell>` 子命令按需生成补全脚本

首版采用新的 `completion` 子命令作为统一导出入口，输出指定 shell 的补全脚本内容，由用户自行重定向到本地补全目录。

选择这个方案的原因：

- 可以直接复用当前 `clap` 命令树，避免手写和维护多套 shell 脚本
- `clap` 生态已经覆盖 `bash`、`zsh`、`fish`，扩展到 `powershell` 的路径也一致
- 用户只需要在安装或升级后执行一次导出，不必让 shell 在每次 `Tab` 时调用 `xgit`

备选方案：

- 手写各 shell 的补全脚本
  - 可控性更高，但维护成本高，且容易与实际命令树漂移
- 采用 shell 的动态回调，在补全时实时执行 `xgit`
  - 能力更强，但会把配置加载和 Git 上下文探测的成本带进每次 `Tab`

### 1.1 新增 `xgit completion --install` 交互式安装路径

`xgit completion --install` 采用“自动识别 + 二次确认”的流程：

- 自动识别当前 shell（优先依据当前终端环境）
- 先在临时目录生成对应补全脚本，提示用户先检查内容
- 明确展示即将写入的目标补全文件路径与 shell 配置文件路径（若该 shell 无需写入 rc 文件也需明确提示）
- 只有用户在交互确认中输入 `Y` 或 `y` 才继续实际写入，否则立即停止且不修改任何目标配置文件
- 配置文件写入采用“托管注释块”策略：使用固定 begin/end 注释包裹补全启用片段，重复执行安装时先定位并替换旧块，避免多次追加重复配置

选择这个方案的原因：

- 兼顾“安装一步到位”和“可审阅、可中止”的安全体验
- 避免用户误以为命令无副作用，同时减少直接改写 shell 配置带来的焦虑
- 与 macOS / Linux 常见 shell 的使用习惯一致

备选方案：

- 执行 `completion --install` 直接写入目标文件，不做确认
  - 体验更快，但风险更高，难以满足用户对可控安装过程的要求
- 每次安装都直接向 rc 文件末尾 append
  - 实现简单，但后续迭代难以稳定升级，会产生重复配置

### 2. completion 生成必须基于有效 CLI 命令树，而不是独立元数据

实现时应继续从运行时配置与语言资源构建完整的 `Command`，再把这棵树同时用于参数解析和补全脚本生成。这样功能开关、现有子命令和本地化说明都来自同一来源。

选择这个方案的原因：

- 新增或修改命令时只需要更新一处
- zsh / fish 等支持描述的 shell 可以自然复用现有本地化文案
- 功能关闭时，completion 至少仍与 help 中可见的命令集合保持一致

备选方案：

- 为补全单独维护一个静态命令描述层
  - 会制造第二份真相源，长期更容易失配

### 3. annotate 的文件候选筛选与脏状态判定都只忽略仓库根 `.xgit/` 路径

`annotate` 的文件发现逻辑必须统一识别“仓库根 `.xgit/` 目录不是注释候选文件”的规则：

- 在 staged / include-untracked 路径中，若候选路径位于仓库根 `.xgit/` 下，系统必须直接排除
- 在 `--latest-commit` 的前置校验中，若 `git status --porcelain` 返回项指向仓库根 `.xgit/` 目录及其子路径，系统必须过滤后再决定是否报“工作区不干净”错误

选择这个方案的原因：

- 能精确解决仓库级配置目录带来的误判
- 能保证 `annotate` 不会把自身配置目录当成待格式化目标
- 不需要要求用户修改 `.gitignore` 或仓库模板
- 过滤范围仅限仓库根 `.xgit/`，不会误伤其他未跟踪文件

备选方案：

- 完全移除 latest-commit 的洁净校验
  - 风险过高，会降低“只处理最后一次提交”的安全边界
- 只在 latest-commit 中忽略 `.xgit/`
  - 不足以覆盖默认 `annotate` 模式下的候选文件筛选问题
- 要求用户把 `.xgit/` 写入 `.gitignore`
  - 侵入用户仓库，不适合作为工具默认前提
- 通过 `git status` pathspec 排除 `.xgit/`
  - 可行，但对命令构造和路径基准更敏感；在代码内解析并过滤 porcelain 结果更直接、可测

### 4. 项目级配置加载逻辑保持不变，忽略策略仅落在 annotate 文件发现与校验层

仓库级 `.xgit/config.toml` 的读取仍继续由配置层按 `<git-root>/.xgit/config.toml` 定位并参与合并，忽略 `.xgit/` 的逻辑不下沉到通用 Git 状态辅助函数或配置定位逻辑中，而是限定在 `annotate` 的候选文件筛选与 `latest-commit` 前置校验。

选择这个方案的原因：

- 可以最大限度减少对现有配置系统的影响
- 符合“忽略 `.xgit/` 只是为了 latest-commit 的洁净判断，而不是把它从仓库视图中抹掉”的语义

备选方案：

- 在通用 Git 状态辅助函数中统一忽略 `.xgit/`
  - 过于宽泛，可能影响其他未来依赖真实仓库状态的命令

### 5. 测试以“输出覆盖 + 安全边界”两条主线扩展

补全能力的测试应覆盖：

- 生成命令能成功输出目标 shell 脚本
- 输出内容至少包含关键子命令与选项，证明它确实来源于当前命令树

annotate 忽略 `.xgit/` 的测试应覆盖：

- 默认 annotate 路径中，当 `.xgit/` 作为未跟踪目录存在时，它不会进入候选文件集合
- 仓库根仅存在 `.xgit/config.toml` 未跟踪时，`--latest-commit` 允许继续执行
- 存在其他未跟踪或已修改源码文件时，命令仍拒绝执行
- 仓库级配置中的有效值仍会参与本次 annotate 运行

## 风险 / 权衡

- `[风险]` 不同 shell 对“显示描述”的支持程度不一致
  - 缓解措施：首版规范只强制补全命令与参数，描述作为“支持的 shell 可用能力”依赖底层生成器自然提供

- `[风险]` 把 `.xgit/` 整体从 annotate 候选与 latest-commit 脏状态中忽略后，用户对该目录中的临时文件变化将不再收到 annotate 相关提示
  - 缓解措施：忽略范围严格限定为仓库根 `.xgit/`，并在设计与测试中确认其他路径仍照常阻塞

- `[权衡]` 首版不做动态 Git 上下文补全
  - 代价：`checkout-remote` 之类命令的参数值仍需用户手输
  - 收益：实现更稳、更跨平台，也避免在每次补全时访问 Git 仓库

## 迁移计划

- 版本升级直接修改 `xgit/Cargo.toml` 到 `2604.2.2`
- 新增 `completion` 子命令后，同步更新 help 文案、README 与测试
- `annotate --latest-commit` 的忽略策略为向后兼容修复，不需要用户迁移已有配置
- 若用户希望启用补全，按文档将 `xgit completion <shell>` 的输出安装到对应 shell 的补全目录即可

## 开放问题

- 首版是否需要把 `powershell` 作为正式文档支持对象，还是仅在实现层保留生成能力
