> Imported historical task context normalized by Transpec for Trellis-first continuation. Use `.trellis/spec/` as the current source of truth; preserved source artifacts remain for provenance.

## 为什么

`xgit` 现在已经提供了多个日常开发命令，但用户在命令名和参数不熟时只能反复查看 help，使用成本逐渐上升。为常用 shell 提供 `Tab` 补全，可以把“记命令”这件事转成“边输入边发现”，更贴合 macOS 和 Linux 上的 CLI 使用习惯。

同时，仓库级配置目录 `.xgit/` 目前会干扰 `annotate` 命令本身的处理范围：它既可能在 `--latest-commit` 模式下被当成工作区不干净的来源，也可能在 `staged` / `include-untracked` 相关路径里被误当成候选文件。需要把仓库级 `.xgit/` 从整个 `annotate` 命令的候选与校验范围中排除，同时保持仓库配置优先级不变。

## 变更内容

- 新增 shell completion 能力，为 `xgit` 提供适用于主流 shell 的补全脚本生成能力，首版至少覆盖 macOS 和 Linux 上常见的 `zsh`、`bash`、`fish`，并为 PowerShell 保留同一生成路径
- 新增 `xgit completion --install` 交互安装能力：自动识别当前终端 shell，先将补全脚本写入临时目录供用户检查，再明确提示将写入的目标补全文件和 shell 配置文件，只有用户输入 `Y` 或 `y` 时才继续安装
- `completion --install` 在写入 shell 配置文件时必须使用带注释边界的托管配置块，后续重复安装时应识别旧块并替换，避免重复追加
- 首版补全至少覆盖命令名、子命令名和静态参数名，优先解决“记不住命令和选项”的核心问题
- 补全输出必须复用现有 CLI 命令树定义，避免维护独立的补全元数据
- 文档需要补充不同 shell 的启用方式，让用户能在本地安装并使用补全
- 将项目版本号更新到 `2604.2.2`
- 修复 `xgit annotate` 命令对仓库根 `.xgit/` 的处理：无论默认 `annotate` 还是 `annotate --latest-commit`，`.xgit/` 及其内容都不得进入注释候选范围，也不得再被视为阻止执行的脏状态来源
- 上述忽略策略只作用于 `annotate` 命令自身的候选筛选与前置校验，不得影响仓库级 `.xgit/config.toml` 继续作为高优先级配置来源被读取

## 功能 (Capabilities)

### 新增功能
- `shell-completion`: 为 `xgit` 生成并分发面向主流 shell 的补全脚本，帮助用户通过 `Tab` 发现命令和参数

### 修改功能
- `annotation-normalization`: 调整 `annotate` 命令的候选筛选与前置校验，忽略仓库级 `.xgit/` 目录，但不改变仓库配置优先级
- `versioning-source-unification`: 将本次落地后的目标版本号更新为 `2604.2.2`

## 影响

- 受影响代码：
  - `xgit/Cargo.toml`
  - `xgit/src/main.rs`
  - `xgit/src/annotate.rs`
  - `xgit/src/config.rs`
  - `xgit/README.md`
- 可能新增补全相关依赖、命令入口、测试与发布/安装说明
- 需要新增 shell completion 规范，并扩展注释规范化与版本规范
