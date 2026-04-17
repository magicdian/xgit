> Imported legacy capability spec normalized by Transpec for Trellis-first maintenance. Use grounded Trellis docs under `.trellis/spec/` for ongoing development; this file preserves historical requirement provenance from the source framework.

# layered-configuration 规范

## 目的
该条目由归档来源变更 `xgit-setup-i18n-and-annotation` 导入。源规范的“目的”字段仍是占位内容；请以下方需求与场景作为当前可追溯的行为定义。
## 需求
### 需求:系统必须支持全局与项目级配置
`xgit` 必须支持默认值、全局配置 `~/.xgit/config.toml` 和项目配置 `<git-root>/.xgit/config.toml`，并在运行时合并为统一配置视图。

#### 场景:项目配置覆盖全局配置
- **当** 全局配置启用了某项功能，而当前 Git 项目的 `.xgit/config.toml` 将同一功能设为关闭
- **那么** 系统必须对该项目优先使用项目配置中的关闭状态

### 需求:项目作用域必须依赖 Git 工作区定位
任何要求编辑或读取项目级配置的命令，必须通过当前 cwd 对应的 Git 工作区根目录定位 `<git-root>/.xgit/config.toml`。

#### 场景:不在 Git 工作区中请求项目配置
- **当** 用户在非 Git 工作区目录中运行需要项目作用域的命令
- **那么** 系统必须返回错误并说明当前不在 Git 工作区内

### 需求:功能开关必须影响命令可用性
配置中的功能使能开关必须决定相关命令是否可执行；至少 `push`、`annotate`、`reset`、`checkout-remote` 与 `completion` 都必须受统一 feature toggle 控制，但 `help` 与 `setup` 命令必须始终可用。

#### 场景:注释功能被关闭
- **当** 有效配置将注释功能设为关闭并且用户执行注释命令
- **那么** 系统必须拒绝执行并说明该功能当前已被配置关闭

#### 场景:reset 功能被关闭
- **当** 有效配置将 `reset` 功能设为关闭并且用户执行 `xgit reset`
- **那么** 系统必须拒绝执行并说明该功能当前已被配置关闭

#### 场景:completion 功能被关闭
- **当** 有效配置将 `completion` 功能设为关闭并且用户执行 `xgit completion zsh`
- **那么** 系统必须拒绝执行并说明该功能当前已被配置关闭

#### 场景:setup 在其他功能关闭时仍可用
- **当** 有效配置关闭了 `push`、`annotate`、`reset`、`checkout-remote` 与 `completion`
- **那么** 用户仍然必须能够运行 `xgit setup` 来修改配置
