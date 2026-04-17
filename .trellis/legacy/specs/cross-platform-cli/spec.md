> Imported legacy capability spec normalized by Transpec for Trellis-first maintenance. Use grounded Trellis docs under `.trellis/spec/` for ongoing development; this file preserves historical requirement provenance from the source framework.

# cross-platform-cli 规范

## 目的
该条目由归档来源变更 `xgit-setup-i18n-and-annotation` 导入。源规范的“目的”字段仍是占位内容；请以下方需求与场景作为当前可追溯的行为定义。
## 需求
### 需求:核心命令必须兼容受支持的平台
`xgit` 的核心命令必须在 macOS、Linux 和 Windows 上可运行，并在这些平台上保持一致的核心行为。

#### 场景:在 macOS 上运行核心命令
- **当** 用户在 macOS 上运行 `xgit help`、`xgit setup` 或核心命令
- **那么** 系统必须能够正常启动并执行对应功能

#### 场景:在 Linux 上运行核心命令
- **当** 用户在 Linux 上运行 `xgit help`、`xgit setup` 或核心命令
- **那么** 系统必须能够正常启动并执行对应功能

#### 场景:在 Windows 上运行核心命令
- **当** 用户在 Windows 上运行 `xgit help`、`xgit setup` 或核心命令
- **那么** 系统必须能够正常启动并执行对应功能

### 需求:路径与外部命令调用不得依赖单一平台假设
系统必须使用跨平台路径与进程调用方式处理 home 目录、配置路径、Git 工作区路径和 Git 命令执行。

#### 场景:解析全局配置路径
- **当** 用户在任一受支持平台上读取或保存全局配置
- **那么** 系统必须将配置解析到该平台用户 home 目录下的 `.xgit/config.toml`

#### 场景:执行 Git 命令
- **当** 系统需要调用 Git 获取工作区、diff 或 remote 信息
- **那么** 系统必须直接调用 Git 进程，而不得依赖特定 shell 的命令拼接语义

### 需求:跨平台实现必须具备测试保障
与平台相关的路径、配置解析、进程调用和命令构建逻辑必须具备单元测试或等效的自动化验证。

#### 场景:验证平台敏感逻辑
- **当** 平台相关逻辑发生改动
- **那么** 系统必须能够通过自动化测试验证该逻辑在受支持平台上的预期行为
