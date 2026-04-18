## 1. 统一分支映射入口

- [x] 1.1 在 `xgit/src/remote.rs` 提供统一的“本地分支到 upstream remote 分支”映射入口，输出 remote 名、remote branch 名和 full upstream ref。
- [x] 1.2 让 `xgit push` 与 `xgit reset` 复用该映射入口，移除调用侧重复解析逻辑。
- [x] 1.3 保持并验证 `--remote`、`--gerrit`、`--no-thin`、`--dry-run` 等现有参数行为不回归。

## 2. Push/Reset 行为修复

- [x] 2.1 在 Gerrit 场景下实现 `refs/for/<branch>` 的分支优先级：未显式 `<branch>` 且存在 upstream 时使用 upstream 远端分支名；显式传入 `<branch>` 时保持用户输入。
- [x] 2.2 校验并固化 `xgit reset` 在本地分支名与 upstream 分支名不同场景下仍以 upstream full ref 为目标。
- [x] 2.3 检查 push 与 reset 在同一分支映射输入下的结果一致性，避免命令间分叉。

## 3. 测试与验收

- [x] 3.1 为 upstream 分支名提取与统一映射入口补充单元测试，覆盖多级分支路径与空值边界。
- [x] 3.2 为 push/Gerrit 和 reset 补充回归测试，覆盖“本地分支名与 upstream 分支名不同”场景。
- [x] 3.3 使用 `xgit push --dry-run` 验证示例场景展开为 `git push origin2 HEAD:refs/for/8676_os6_xpdev_androidB`，并完成 reset 对应场景验证。

## 4. 版本与发布

- [x] 4.1 将 `xgit/Cargo.toml` 的 `package.version` 更新为 `2604.4.1`。
- [x] 4.2 检查帮助文案与错误提示无新增歧义，并整理变更说明用于提交。
