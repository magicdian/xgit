## 新增需求
<!-- 无 -->

## 修改需求

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

## 移除需求
<!-- 无 -->
