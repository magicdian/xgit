# tasks for xgit-auto-remote-push

## 概览
实现 `xgit push` 的最小可交付清单（MVP），可在实现阶段拆分为具体 PRs。

## 任务清单
1. 初始化项目骨架（apply 时执行）
   - [x] 创建 Rust 项目 `xgit`（`cargo init --bin`）
   - [x] 添加依赖：`clap`, `anyhow`, `thiserror`, `dirs`, `serde`（如需配置）
   - [x] 创建 `README.md` 与基础目录结构
   - 估时：0.5 天

2. CLI 与参数解析
   - [x] 使用 `clap` 定义 `push` 子命令及选项：`--remote`, `--gerrit`, `--no-thin`, `--force-with-lease`, `--dry-run`, `--verbose`
   - [x] 添加 help 与 usage 示例
   - 估时：0.5 天

3. 远端识别模块
   - 实现读取 git 配置与命令输出的 wrapper（调用 `git config`、`git rev-parse`、`git remote -v`）
   - 实现优先级逻辑（分支配置 → ENV/配置 → 自动推断 → 失败处理）
   - 提供可测试的接口以便单元测试
   - 估时：1 天

4. Gerrit 检测与 refspec 构造
   - 实现 Gerrit 判定（URL 关键字/端口检测 + 强制标志）
   - 实现 refspec 构造（包含 `refs/for/`）
   - 估时：0.5 天

5. Push 执行层
   - [x] 根据构造的参数执行 `git push`（通过 `std::process::Command`），转发 stdout/stderr
   - [x] 支持 `--dry-run`、`--no-thin`、`--force-with-lease`
   - 估时：0.5 天

6. 测试
   - [ ] 单元测试：远端识别与命令构造
   - [ ] 集成测试：在临时仓库中验证命令（`--dry-run`）
   - 估时：1 天

7. 文档与发布
   - 完善 `README.md`、示例与常见问题（如 remote 冲突处理）
   - 打包发布（例如 GitHub release）
   - 估时：0.5 天

## 验收准则
- `xgit push` 在典型本地仓库能自动选择 remote 并在 `--dry-run` 下打印正确的 `git push` 命令
- 支持 `--no-thin`，命令在 `--dry-run` 下包含该参数
- 出现 ambiguous remote 时，非交互模式返回明确错误并提示 `--remote`

## 后续可选任务
- 支持 `push-option`、`topic` 与更细粒度的 Gerrit 集成
- 提供 `xgit doctor` 检查 remote 与配置一致性
- VSCode/JetBrains 插件集成
