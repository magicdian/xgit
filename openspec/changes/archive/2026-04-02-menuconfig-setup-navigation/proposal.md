## 为什么

当前 `xgit setup` 虽然名义上是 menuconfig 风格界面，但主体仍然是“左侧栏目 + 右侧平铺字段”的双栏表单；随着注释配置不断扩展，尤其是 `annotate` 栏目已经堆叠出大量字段，用户很难按任务心智逐级进入和返回，也很难快速理解某项配置属于哪一层。

同时，现有配置模型还存在两个会继续拖累 setup 可用性的问题：功能开关只覆盖 `push` 与 `annotate`，还没有纳入 `reset`、`checkout-remote`、`completion`；注释上下文配置仍然围绕固定的 `reference_kind` / `reference_value` 语义展开，setup 只能暴露扁平 CSV，而不能表达更结构化、更可定制的引用字段与引用类型。

## 变更内容

- 将 `xgit setup` 从固定双栏字段表单重构为真正的单栏、逐级进入/逐级返回的 menuconfig 式导航界面。
- 把现有设置项重组为层级配置树，至少覆盖“通用设置 / 身份设置 / 注释设置”等一级入口，并把注释相关配置继续拆分为多个二级或三级子菜单，而不是继续平铺在单页字段列表中。
- 将当前“代码文件类型”选择器从 setup 的特例弹窗提升为通用层级导航模型中的标准子菜单能力，统一 `Enter`、`Space`、`Esc` 的交互语义。
- 扩展 feature toggle 配置模型与 setup 入口，除 `push`、`annotate` 外，新增对 `reset`、`checkout-remote` 与 `completion` 的开关管理，并要求 help 与命令执行链路遵循这些开关状态。
- 重构注释设置的配置组织方式，提供更结构化的入口来管理：
  - 引用与表单字段
  - 代码文件类型
  - 渲染行为
  - `add` / `modify` / `del` 模板
  - 旧代码展示策略
- 调整注释上下文配置模型，使 setup 不再只围绕固定的 `reference_kind` / `reference_value` 文本对进行编辑，而是支持可配置的引用字段与引用类型定义，为后续完全自定义上下文字段留出稳定模型。

## 功能 (Capabilities)

### 新增功能
<!-- 无 -->

### 修改功能
- `interactive-setup`: 将 setup 从双栏字段列表改为单栏树形 menuconfig 导航，并引入更细粒度的层级菜单结构
- `layered-configuration`: 扩展 feature toggle 配置模型，使 `reset`、`checkout-remote` 与 `completion` 也受统一配置层控制
- `annotation-normalization`: 调整注释运行时上下文与 setup 对应的配置模型，支持更结构化的引用字段/引用类型定义，而不是把 setup 绑定到固定的 `reference_kind` / `reference_value`
- `remote-tracking-branch-ops`: 让 `xgit reset` 与 `xgit checkout-remote` 在对应功能开关关闭时明确拒绝执行
- `shell-completion`: 让 `xgit completion` 在对应功能开关关闭时明确拒绝执行，并在 help 中显示禁用状态

## 影响

- `xgit/src/setup_ui.rs` 需要从 `(section, field)` 索引驱动的表单状态机重构为树节点驱动的导航状态机。
- `xgit/src/config.rs` 与默认配置需要扩展 feature toggle 模型，并重新整理 annotate 相关可配置结构。
- `xgit/src/main.rs`、`xgit/src/annotate.rs` 与对应命令测试需要接入新的功能开关与注释上下文配置模型。
- `xgit/resources/i18n/*.toml` 需要补充新的菜单节点、breadcrumb、状态提示与注释配置文案。
- 相关 specs、README 与 setup/annotate/command 行为测试需要同步更新，以反映新的导航方式与配置边界。
