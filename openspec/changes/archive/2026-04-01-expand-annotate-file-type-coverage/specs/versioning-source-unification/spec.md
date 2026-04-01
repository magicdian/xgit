## ADDED Requirements

## MODIFIED Requirements

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

## REMOVED Requirements
