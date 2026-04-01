# versioning-source-unification 规范

## 目的
待定 - 由归档变更 add-unified-version-constant 创建。归档后请更新目的。
## 需求
### 需求:系统必须以 Cargo package.version 作为唯一版本来源
系统必须将 `Cargo.toml` 中的 `package.version` 作为项目版本号的唯一来源；系统禁止在多个模块中直接硬编码版本号字面量。

#### 场景:读取版本号时命中 Cargo 版本来源
- **当** 任意模块需要获取当前业务版本号
- **那么** 该模块必须从 Cargo 版本访问入口读取版本值
- **并且** 系统不得在业务逻辑中直接拼写版本号字面量

### 需求:CLI 必须通过 Cargo 版本来源展示版本信息
CLI 必须提供标准版本查询入口，并且该入口输出的版本号必须来自 `package.version`，而非独立硬编码文本。

#### 场景:查询 CLI 版本号
- **当** 用户执行版本查询命令（如 `xgit --version`）
- **那么** 输出中必须包含 `Cargo.toml` 当前 `package.version` 的值

#### 场景:更新 Cargo 版本后 CLI 输出同步变化
- **当** 维护者修改 `Cargo.toml` 的 `package.version`
- **那么** `xgit --version` 的输出必须随之变化且无需额外修改代码常量

#### 场景:本次版本值采用方案 2
- **当** 本变更落地后维护者查看 `Cargo.toml`
- **那么** `package.version` 必须设置为 `2604.1.3`（语义对应 `2604.01.3`）

### 需求:系统必须支持 Cargo 兼容版本约定统一
系统必须允许团队采用自定义版本命名约定，但最终版本字符串必须兼容 Cargo 版本格式约束。

#### 场景:采用日期化版本约定
- **当** 团队希望使用 `YYMM.DD.BuildNumber` 的日期化版本策略
- **那么** 系统必须要求其转换为 Cargo 可接受的等价格式后再写入 `package.version`
