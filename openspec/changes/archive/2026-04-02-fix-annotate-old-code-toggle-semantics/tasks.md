## 1. 收敛 annotate 渲染判定

- [x] 1.1 在 `xgit/src/annotate.rs` 中集中实现 `modify` / `del` 模板开关与旧代码处理开关的前置条件判定。
- [x] 1.2 调整 `modify` 渲染逻辑：当旧代码处理关闭但 `modify` 模板开启时，仍生成修改注释，同时移除 `old` 字段与旧代码正文残留。
- [x] 1.3 调整 `del` 渲染逻辑：只有 `del` 模板和旧代码处理同时开启时才渲染删除注释，否则把删除候选标记为未格式化并返回明确原因。

## 2. 对齐 setup 帮助说明

- [x] 2.1 更新 `xgit/src/setup_ui.rs` 中 `修改格式`、`删除格式` 与 `旧代码处理` 的上下文帮助生成逻辑，使其反映新的开关矩阵。
- [x] 2.2 更新 `xgit/resources/i18n/zh-CN.toml` 与 `xgit/resources/i18n/en-US.toml` 中相关帮助文案，明确说明“修改格式可独立生效”和“删除格式依赖旧代码处理”。

## 3. 回归验证

- [x] 3.1 为 `modify` / `del` 与旧代码处理组合补充单元测试，覆盖“modify 开启 + old 关闭”、“modify 关闭 + old 开启”、“del 开启 + old 关闭”和“del 开启 + old 开启”。
- [x] 3.2 为 setup 帮助内容补充测试或快照验证，确认三个菜单项都能展示正确的依赖说明。
- [x] 3.3 运行与 annotate、setup 相关的测试集，确认不再出现开关关闭后仍输出 `modify` / `old` 的回归。
