> Imported historical task context normalized by Transpec for Trellis-first continuation. Use `.trellis/spec/` as the current source of truth; preserved source artifacts remain for provenance.

## 为什么

`menuconfig-setup-navigation` 把 `新增格式`、`修改格式`、`删除格式` 和 `旧代码处理` 拆成了“开关 + 子菜单”的结构，但当前 setup 在关闭这些项时会直接改写它们的关联配置值，而不只是切换启用状态。这会让用户在重新开启后丢失自己编辑过的模板文本或模式选择，导致“开关状态”和“配置值”这两个本应独立的属性发生串扰。

目前已经确认两类具体问题：`add` / `modify` / `del` 模板在关闭时会被重置为默认模板；`旧代码处理` 在关闭后重新开启时会丢失此前选择的 `line_comment` / `block_comment` 模式并退回 legacy 模式。这个问题直接影响 setup 的可预期性，也会让 annotate 的后续输出悄悄偏离用户原本保存的配置。

## 变更内容

- 调整 setup 中“开关 + 子菜单”配置项的语义：关闭某项时只能改变该项的启用状态，不得顺带把其关联值恢复为默认值、空值或其他隐式回退值。
- 修正 `新增格式`、`修改格式`、`删除格式` 的开关行为：关闭时仅切换 `enabled`，重新开启时必须保留用户此前编辑过的 `start` / `end` 模板内容。
- 修正 `旧代码处理` 的开关行为：关闭时仅表示该能力当前禁用，重新开启时必须恢复用户关闭前最后一次选中的旧代码模式，而不是固定回到 legacy 模式。
- 对现有 setup 开关项做一次同类审查，并补充回归约束，确保其他开关项不会出现“关闭时覆盖关联值”的同类问题。

## 功能 (Capabilities)

### 新增功能

<!-- 无 -->

### 修改功能

- `interactive-setup`: 约束 setup 中带有关联配置值的开关项必须保留其已编辑值，禁止在关闭或重新开启时隐式重置模板、模式或其他子配置。

## 影响

- `xgit/src/setup_ui.rs` 中开关切换逻辑、旧代码处理启停逻辑以及相关菜单状态管理需要调整。
- `xgit/src/config.rs` 中旧代码处理的配置建模、兼容加载/保存语义可能需要调整，以支持“禁用但保留上次模式”。
- `xgit/src/setup_ui.rs` 的测试需要补充，覆盖模板开关与旧代码处理开关在“关闭 -> 开启”往返后的值保持行为。
- `interactive-setup` 的增量规范需要明确“开关状态”和“关联配置值”解耦的行为契约。
