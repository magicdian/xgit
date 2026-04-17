> Imported historical task context normalized by Transpec for Trellis-first continuation. Use `.trellis/spec/` as the current source of truth; preserved source artifacts remain for provenance.

## 为什么

`xgit` 当前只有静态的 `push` 子命令，帮助信息、错误文案和命令说明直接写在代码里，缺少全局/项目级配置层，也无法承载“按团队规范自动补充注释”的复用能力。随着工具从单一 push 助手演进为可配置的开发辅助工具，现在需要补齐运行时多语言、分层配置、交互式设置界面和可策略化的规范注释能力。

## 变更内容

- 为 `xgit` 引入资源驱动的运行时本地化机制，统一管理 help、错误信息、状态提示和设置界面文案，支持中文和英文，禁止在业务代码中硬编码最终展示文本。
- 为 `xgit` 引入分层配置系统，支持默认值、全局配置 `~/.xgit/config.toml` 和项目配置 `<git-root>/.xgit/config.toml`，项目配置优先于全局配置。
- 新增 `xgit setup` 的 ratatui 设置界面，支持编辑全局配置和项目配置；`xgit setup --project` 必须在 Git 工作区内运行，否则报错。
- 在设置界面中提供功能使能开关、push 设置占位项、规范化注释策略设置、文件类型关联和运行时表单字段配置。
- 新增规范化注释功能，基于配置的策略包和运行时上下文表单工作，支持按 `add` / `modify` / `del` 三类改动生成注释，默认处理 staged 变更，并支持 `--latest-commit` 处理最后一个提交的内容。
- 为规范化注释功能预留多文件类型渲染器扩展点；第一阶段聚焦 C-like 行注释块，特殊文件格式仅要求具备可扩展的模型和跳过/提示能力，不要求全部落地。
- 明确 `xgit` 必须兼容 macOS、Linux 和 Windows 三个平台，命令、配置路径解析、终端交互和文件处理逻辑不得依赖单一平台的 shell 或路径假设。
- 为配置解析、本地化、设置界面状态、变更源识别和注释渲染建立完备的单元测试，并补齐跨平台兼容性的验证策略。

## 功能 (Capabilities)

### 新增功能
- `localized-cli`: 运行时按语言资源生成 CLI help、错误信息、状态提示和功能关闭提示。
- `layered-configuration`: 解析默认值、全局配置和项目配置，并对所有命令暴露统一的配置结果与功能使能状态。
- `interactive-setup`: 提供 menuconfig 风格的 ratatui 设置界面，支持编辑全局和项目级配置。
- `annotation-normalization`: 基于变更源、运行时上下文表单、策略模板和文件渲染器生成规范化注释。
- `cross-platform-cli`: 保证 `xgit` 在 macOS、Linux 和 Windows 上具有一致的核心行为与兼容性约束。

### 修改功能
<!-- 无 -->

## 影响

- `xgit` CLI 启动流程将从静态 `clap derive` 扩展为“bootstrap 解析 + 配置加载 + 语言资源加载 + 运行时命令树生成”。
- `push` 功能会接入统一配置层和功能开关，并在 help 中展示功能状态。
- 将引入新的依赖和目录结构，用于处理 TOML 配置、语言资源、ratatui 界面和注释策略。
- 将新增对 Git 工作区定位、staged/latest-commit diff 源读取、注释渲染与应用的实现与测试。
- 实现需要显式处理跨平台路径、终端、home 目录定位和 Git 调用兼容性，并建立对应的测试矩阵与单元测试覆盖。
