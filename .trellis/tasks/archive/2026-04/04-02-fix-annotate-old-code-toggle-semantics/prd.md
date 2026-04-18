> Imported historical task context normalized by Transpec for Trellis-first continuation. Use `.trellis/spec/` as the current source of truth; preserved source artifacts remain for provenance.

## 为什么

`menuconfig-setup-navigation` 把 `modify` / `del` 模板和“旧代码处理”拆成了独立入口，但当前规范没有把它们的优先级和联动边界定义清楚，导致配置界面里看起来已经关闭的选项，在实际格式化后仍然可能输出 `modify` 注释、`old=` 字段或 legacy `old:` 注释。这样会让用户无法根据 setup 中的开关可靠预测 annotate 的结果。

同时，删除格式天然依赖旧代码展示，但现在帮助栏没有明确说明这层前置条件；用户即使看到了 `删除格式` 和 `旧代码处理` 两个入口，也不知道只有两者同时开启时删除块格式化才真正生效。

## 变更内容

- 明确定义 `modify` 模板开关与旧代码处理开关的运行时语义边界：
  - 仅关闭旧代码处理时，`modify` 变更仍可生成修改注释，但不得输出 `old=` 字段、legacy `old:` 注释或其他旧代码内容。
  - 仅开启旧代码处理而关闭 `modify` 模板时，系统不得为修改块补充旧代码注释，也不得因为旧代码处理处于开启状态而“反向激活”修改注释。
- 明确定义 `del` 模板与旧代码处理的依赖关系：只有在删除模板开启且旧代码处理开启时，删除块格式化才允许生效；如果任一侧关闭，系统必须按未满足前置条件处理删除块，而不是继续输出带旧代码的删除注释。
- 调整 annotate 的旧代码兼容回退语义，禁止在配置已明确关闭相关能力时仍通过 `{old}` 占位符或 legacy fallback 输出旧代码内容。
- 为 setup 帮助栏补充开关联动说明，至少明确提示：
  - `删除格式` 依赖 `旧代码处理`；
  - `修改格式` 在旧代码处理关闭时仍可生效，但不会带出旧代码字段；
  - `旧代码处理` 本身不会替代 `modify` / `del` 模板开关。

## 功能 (Capabilities)

### 新增功能

<!-- 无 -->

### 修改功能

- `annotation-normalization`: 收紧 `modify` / `del` 模板与旧代码处理之间的联动语义，移除“开关已关闭但仍输出 old 内容”的兼容回退行为。
- `interactive-setup`: 为 `修改格式`、`删除格式` 与 `旧代码处理` 提供明确的一致性说明，让帮助栏直接反映这些依赖关系和生效条件。

## 影响

- `xgit/src/annotate.rs` 中的变更块渲染逻辑、legacy old-code fallback 与对应测试矩阵需要调整。
- `xgit/src/setup_ui.rs` 的帮助栏摘要、状态提示和菜单说明需要补充开关联动文案。
- `xgit/resources/i18n/zh-CN.toml` 与 `xgit/resources/i18n/en-US.toml` 需要新增或修改帮助文案键。
- `annotation-normalization` 与 `interactive-setup` 的增量规范需要同步更新，以固定这些开关组合的预期行为。
