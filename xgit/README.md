xgit — 可配置的 Git 开发辅助工具

## 版本维护
- 单一版本来源：`Cargo.toml` 的 `[package].version`
- CLI 版本输出（`xgit --version`）直接读取 Cargo 包版本
- 推荐可以使用日期化命名约定 `YYMM.DD.BuildNumber`，但必须保持 Cargo 兼容；当前落地写法：`2604.2.1`（语义对应 `2604.02.1`）

## 主要能力
- `push`：自动识别 remote 并执行推送
- `setup`：基于 ratatui 的 menuconfig 风格配置界面
- `annotate`：基于 staged/latest-commit 改动生成规范化注释块预览
- `reset`：将当前本地分支重置到其 upstream 跟踪分支
- `checkout-remote`：从远端跟踪分支创建本地分支

## 配置位置与优先级
- 全局配置：`~/.xgit/config.toml`
- 项目配置：`<git-root>/.xgit/config.toml`
- 优先级：默认值 < 全局配置 < 项目配置 < 环境变量 < 命令参数（非语言项）

## 语言切换
- 支持 `zh-CN` 与 `en-US`
- 环境变量：`XGIT_LANG`
- 配置项：`[ui].lang`

## setup 用法
- 全局配置：`xgit setup`
- 项目配置：`xgit setup --project`
- 关键操作：
  - 主页面：`Up/Down` 选择栏目，`Enter` 进入
  - 子页面：`Up/Down` 切换字段，`Enter` 或 `Left/Right` 切换布尔值
  - `代码文件类型`：`Enter` 打开层级选择器；分类层 `Space` 整类切换，子项层 `Space` 单项切换，`Enter/Esc` 返回
  - 子页面：`ESC` 返回主页面
  - 主页面：`ESC` 退出（有未保存修改时会弹出保存确认）
  - `e` 编辑文本字段
  - `s` 保存配置
  - `q` 退出（与 `ESC` 一致）

## push 用法
- `xgit push`
- `xgit push <branch>`
- 常用参数：
  - `--remote <name>`
  - `--gerrit`
  - `--no-thin`
  - `--force-with-lease`
  - `--dry-run`

## reset 用法
- `xgit reset`
- `xgit reset --hard`
- 说明：
  - 仅在当前已切到本地分支且该分支已配置 upstream tracking branch 时可执行
  - `--hard` 会映射到 `git reset --hard <upstream>`，会覆盖工作区与暂存区内容，请谨慎使用
  - 若当前为 detached HEAD 或没有 upstream，会直接拒绝执行并给出提示

## checkout-remote 用法
- `xgit checkout-remote <remote-branch>`
- `xgit checkout-remote <remote-branch> <local-branch>`
- 示例：
  - `xgit checkout-remote 8676_os6_xpdev`
  - `xgit checkout-remote 8676_os6_xpdev 8676_os6_xpdev_clean`
- 说明：
  - 只传一个参数时，默认本地分支名与远端分支名相同
  - 命令会先检查本地目标分支是否存在；若存在则拒绝执行
  - remote 解析采用“首选 remote（`XGIT_REMOTE` / `git config xgit.remote` / 常见默认 remote）优先，找不到时回退候选扫描”的策略
  - 若候选 remote branch 不存在或存在多个同名候选，命令会报错而不是静默选择

## 注释规范化流程
默认使用 staged 改动，也可以显式指定 latest-commit：

```bash
xgit annotate --staged --reason "bugfix" --reference-kind bug --reference-value XP-123
xgit annotate --latest-commit --reason "refactor" --reference-kind req --reference-value REQ-88
```

说明：
- `--staged` 模式可通过配置 `annotate.staged.include_untracked` 或参数 `--include-untracked` 纳入未跟踪文件
- `--latest-commit` 模式会校验：
  - 非根提交
  - 非 merge commit
  - 工作区干净
- setup 中代码文件类型默认启用 `C/C++` 与 `Java`，并可按分类启用 `JavaScript`、`Rust`、`Kotlin`
- 渲染器按文件规则分发；当前完整实现 `c_line_block`（上述内置类型均可映射到该渲染器）
- 注释渲染可通过配置项控制：
  - `annotate.render.align_with_code_indent`：注释是否对齐代码缩进（默认 `false`）
  - `annotate.render.wrap_blank_lines`：注释块是否包裹变更中的空白行（默认 `true`）

## 跨平台支持
- 目标平台：macOS、Linux、Windows
- 关键实现约束：
  - 路径统一使用跨平台 `PathBuf`
  - Git 调用统一使用 `std::process::Command`
- CI：`.github/workflows/xgit-ci.yml` 提供三平台 build/test 矩阵

## 本地构建
```bash
cargo build --release
cargo test
```
