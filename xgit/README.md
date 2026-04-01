xgit — 可配置的 Git 开发辅助工具

## 主要能力
- `push`：自动识别 remote 并执行推送
- `setup`：基于 ratatui 的 menuconfig 风格配置界面
- `annotate`：基于 staged/latest-commit 改动生成规范化注释块预览

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
