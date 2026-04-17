## 1. Shell completion 命令接入

- [x] 1.1 在 `xgit/Cargo.toml` 中补充 completion 生成所需依赖，并在 `xgit/src/main.rs` 中新增 `completion <shell>` 子命令定义
- [x] 1.2 实现基于现有 `clap` 命令树的 completion 脚本输出逻辑，覆盖首版支持的 shell 类型与不支持 shell 的报错
- [x] 1.3 在 `xgit/resources/i18n/zh-CN.toml` 与 `xgit/resources/i18n/en-US.toml` 中补充 completion 子命令相关 help / 错误文案
- [x] 1.4 扩展 `xgit completion --install`：自动识别当前 shell、先生成 tmp 脚本并提示检查、展示目标补全文件/配置文件写入路径，并在用户输入 `Y/y` 后才执行写入
- [x] 1.5 `completion --install` 写入 shell 配置时使用带 begin/end 注释的托管块，并支持识别旧块后替换，避免重复追加

## 2. annotate .xgit 忽略修复

- [x] 2.1 在 `xgit/src/annotate.rs` 中抽取 annotate 候选文件筛选与 latest-commit 工作区洁净校验所需的 `.xgit/` 过滤逻辑，只忽略仓库根 `.xgit/` 目录及其子路径
- [x] 2.2 保持 `xgit/src/config.rs` 的项目级配置读取逻辑不变，并让默认 `annotate` 与 `annotate --latest-commit` 在忽略 `.xgit/` 后继续使用仓库级有效配置

## 3. 测试、文档与版本

- [x] 3.1 为 completion 输出补充自动化验证，至少覆盖受支持 shell 的成功生成与未知 shell 的失败路径
- [x] 3.2 为 `annotate` 补充回归测试，验证默认模式不会把 `.xgit/` 纳入候选、`--latest-commit` 在仅存在 `.xgit/` 状态时可继续执行、存在其他脏文件时仍会失败，并验证仓库级配置仍然生效
- [x] 3.3 更新 `xgit/README.md`，补充 shell completion 的生成/启用方式与 `.xgit/` 忽略修复后的 annotate 使用说明
- [x] 3.4 将 `xgit/Cargo.toml` 版本更新到 `2604.2.2`，并验证 `xgit --version` 输出同步变化
- [x] 3.5 为 `completion --install` 补充自动化验证，至少覆盖 shell 自动识别、确认前不写入、确认后写入与目标路径提示
- [x] 3.6 为托管配置块补充自动化验证，至少覆盖“首次写入成功”和“重复安装替换旧块而非重复追加”
