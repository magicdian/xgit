## 1. CLI 与文案接入

- [x] 1.1 在 `xgit/src/main.rs` 中新增 `reset` 与 `checkout-remote` 子命令及参数定义
- [x] 1.2 在 `xgit/resources/i18n/zh-CN.toml` 与 `xgit/resources/i18n/en-US.toml` 中补充 help、错误与状态文案

## 2. Git 分支关系解析

- [x] 2.1 在 `xgit/src/remote.rs` 中新增“当前本地分支 / detached HEAD”识别 helper
- [x] 2.2 在 `xgit/src/remote.rs` 中新增 upstream remote branch 解析 helper，返回完整 `<remote>/<branch>` 引用
- [x] 2.3 在 `xgit/src/remote.rs` 中新增本地分支存在性、remote tracking branch 存在性与 remote branch 候选枚举 helper

## 3. 命令执行逻辑

- [x] 3.1 实现 `execute_reset`，覆盖 upstream 校验、`--hard` 传递与 Git 命令执行
- [x] 3.2 实现 `execute_checkout_remote`，覆盖参数缺省、本地重名分支拦截、remote 解析与 `git checkout -b` 执行
- [x] 3.3 复用或抽取现有 remote 偏好逻辑，使 `checkout-remote` 支持“首选 remote + 候选回退 + 歧义报错”

## 4. 测试与文档

- [x] 4.1 为 upstream 解析、remote 候选选择和本地/远端分支存在性检查补充单元测试
- [x] 4.2 为 `reset` 与 `checkout-remote` 的成功/失败路径补充集成测试或等效自动化验证
- [x] 4.3 更新 `xgit/README.md`，补充两个新命令的用法、示例与失败提示
- [x] 4.4 更新 `xgit/Cargo.toml` 版本到 `2604.2.1`，并验证 `xgit --version` 输出同步变化
