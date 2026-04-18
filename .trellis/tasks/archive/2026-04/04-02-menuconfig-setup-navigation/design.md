> Imported historical task context normalized by Transpec for Trellis-first continuation. Use `.trellis/spec/` as the current source of truth; preserved source artifacts remain for provenance.

## 上下文

当前 `xgit setup` 的交互骨架仍然是“双栏栏目/字段表单”：

- 左栏通过 `SECTION_KEYS` 固定 5 个一级栏目
- 右栏通过 `field_count`、`field_lines`、`toggle_field`、`get_text`、`apply_text` 等函数，按 `(section, field)` 索引硬编码字段数量、显示文本和编辑行为
- `代码文件类型` 是唯一被抽成层级选择器的特例弹窗

这套结构在配置项较少时还能工作，但一旦注释配置继续扩展，就会出现几个明显问题：

- 用户只能在一个很长的字段列表里上下移动，缺少“按任务逐级进入”的信息架构。
- 新增一个 setup 字段通常要同时修改多个索引驱动函数，维护成本高，也容易引入错位。
- `code_file_types` 已经表现出 setup 需要层级子菜单，但当前状态机没有通用的树节点模型，只能继续靠特例扩展。
- `features` 当前只覆盖 `push` / `annotate`，无法承载用户已经提出的 `reset` / `checkout-remote` / `completion` 开关。
- `annotate` 的运行时上下文虽然在概念上已经是“按表单采集”，但 setup 仍然只能编辑扁平的 `fields csv + reference_kinds csv`，没有办法表达真正结构化的字段定义，也会把 setup 继续绑死在 `reference_kind` / `reference_value` 这组历史语义上。

本次变更的利益相关者主要有两类：

- `xgit` 的直接用户：希望 setup 像 `menuconfig` 一样靠上下、空格、回车、返回就能完成配置，而不是理解一页长表单
- 维护者：需要一个可扩展的 setup 架构，避免每次新增配置都去手工调整索引和字段分发表

## 目标 / 非目标

**目标：**

- 把 `xgit setup` 重构为真正的单栏、树形、逐级进入/逐级返回的 menuconfig 式导航。
- 用数据驱动的菜单节点模型替代当前 `(section, field)` 硬编码表单模型。
- 让 `代码文件类型` 成为通用树节点系统中的标准多选子菜单，而不是永久保留为特例弹窗。
- 扩展 feature toggle，使 `reset`、`checkout-remote`、`completion` 进入统一配置模型，并影响 help 与命令可执行性。
- 为注释上下文字段建立结构化定义模型，让 setup 可以配置字段标签、字段类型和引用类型集合，而不再只暴露硬编码 CSV。
- 保持对旧配置的兼容读取，并允许 setup 在用户保存后写回新的规范化结构。

**非目标：**

- 不在本次变更中重写 ratatui、crossterm 或整体终端后端。
- 不在本次变更中引入搜索框、模糊过滤、鼠标交互或多窗口复杂布局。
- 不在本次变更中把 annotate 变成任意动态表单系统；只支持有限的标量字段类型，例如 `text` 与 `single-select`。
- 不在本次变更中新增自定义 CLI 参数来为任意 annotate 字段赋值；命令行入口仍保留现有固定参数，新增字段先通过交互式输入采集。
- 不在本次变更中处理任意深度的配置 schema 自动生成；菜单树仍由代码装配，但节点与行为必须统一建模。

## 决策

### 决策 1：用树节点状态机替代 `(section, field)` 索引状态机

新的 setup 交互不再维护 `section`、`field` 与 `Focus::Menu/Fields` 这套双阶段焦点模型，而是维护一条“当前所在菜单路径”和每层菜单的选中索引。核心状态将接近：

```text
SetupState
├── stack: Vec<MenuFrame>
│   ├── key: "root"
│   ├── title: "通用设置"
│   └── selected: 2
├── editing: Option<EditorState>
├── confirm_exit: Option<ConfirmState>
└── dirty / status
```

每个菜单项统一抽象为节点：

- `submenu`
- `toggle`
- `choice`
- `text`
- `multiselect`
- `action`（如果后续需要）

对应交互规则：

- `Up/Down`：移动当前层选中项
- `Enter`：进入 `submenu`，或开始编辑 `text`，或在 `choice` 节点上循环/展开
- `Space`：切换 `toggle`，或切换 `multiselect` 的当前项
- `Esc`：返回上一级；若已在根节点且存在脏修改，则进入退出确认

选择这个方案的原因：

- 它天然匹配 `menuconfig` 的心智模型，不需要先在“菜单模式”和“字段模式”之间切焦点。
- 交互语义可以对所有节点统一定义，`代码文件类型` 不再是唯一特例。
- 新增配置项时只需要为树增加节点，不再同时改 5 个索引分发表函数。

备选方案：

- 保留双栏布局，只在 `annotate` 下增加更多弹窗：短期改动小，但问题会继续累积。
- 完全自动从配置 schema 生成 setup：抽象更强，但对当前项目规模过重，也会拖慢本次落地。

### 决策 2：界面采用单栏主体 + breadcrumb/header + footer help/status

本次重构不追求完全复刻 Linux `menuconfig` 的视觉样式，但会保留其核心信息结构：单一主体列表，用户只处理当前层级。

建议布局：

```text
┌──────────────────────────────────────────────┐
│ xgit setup > 注释设置 > 修改格式              │
├──────────────────────────────────────────────┤
│ [ ] 启用自定义修改格式                        │
│ ---> 起始模板                                │
│ ---> 结束模板                                │
├──────────────────────────────────────────────┤
│ Enter 进入/编辑   Space 切换   Esc 返回       │
│ 状态：已进入“修改格式”                        │
└──────────────────────────────────────────────┘
```

这里的关键变化是：

- 去掉左栏固定栏目列表，当前层只显示一个列表
- 顶部用 breadcrumb 明确“我现在在哪一层”
- 底部 help 只显示当前层相关操作，不再同时解释“左栏”和“右栏”两套焦点

选择这个方案的原因：

- 单栏能最大化保留 menuconfig 的“只聚焦当前层”的优点。
- breadcrumb 能弥补双栏取消后对全局位置感知的损失。
- 布局简化后，更容易在中英文下保持宽度和可读性稳定。

备选方案：

- 保留左栏作为全局导航，右栏只展示当前层：视觉上折中，但仍然会干扰“逐级进入”的体验。
- 完全全屏弹窗式逐层跳转：层级感更强，但会让退出确认、编辑态和状态提示更难组织。

### 决策 3：把 feature toggle 扩展为统一的命令能力开关

`FeaturesConfig` 从当前的：

- `push`
- `annotate`

扩展为至少：

- `push`
- `annotate`
- `reset`
- `checkout_remote`
- `completion`

行为规则保持一致：

- 命令 help 中必须标出禁用状态
- 命令执行时必须在进入业务逻辑前拒绝执行
- `help` 与 `setup` 永远不受 feature gate 关闭影响

选择这个方案的原因：

- 用户已经明确把这些命令视为“可开关的功能”，setup 必须能完整表达。
- 统一的 feature gate 能让 setup、help、命令执行三处语义保持一致。

备选方案：

- 只在 setup 中显示这些开关，但暂不接入命令执行：会制造“界面能关、命令仍可跑”的不一致。
- 为不同命令各自新增零散开关逻辑：会破坏当前 layered-configuration 的统一性。

### 决策 4：注释上下文字段改为结构化字段定义，而不是 CSV + 硬编码特殊字段

为避免 setup 继续写死 `reference_kind` / `reference_value`，本次设计把 annotate 表单改为“字段定义 + 可选项集合”的结构化模型。建议的规范化方向如下：

```toml
[[annotate.form.fields]]
id = "reason"
label = "原因"
kind = "text"
required = true

[[annotate.form.fields]]
id = "reference_kind"
label = "引用类型"
kind = "single-select"
option_set = "reference_kinds"
required = true

[[annotate.form.fields]]
id = "reference_value"
label = "引用值"
kind = "text"
required = true

[annotate.form.option_sets.reference_kinds]
values = ["bug", "req"]
```

这里的关键点是：

- setup 编辑的是字段定义，而不是逗号分隔字符串
- `reference_kind` 只是默认字段 ID，不再是 setup 内部唯一认识的特殊概念
- 新增字段时，只要定义 `id/label/kind` 即可加入表单
- 对 `single-select` 字段，其候选值来自命名 option set，可由 setup 进入子菜单维护

模板占位符策略：

- 继续保留现有 `{reason}`、`{reference_kind}`、`{reference_value}` 兼容语义
- 对任意自定义字段，直接使用字段 `id` 作为占位符名，例如 `{ticket}`、`{module}`

这样可以避免再次引入新的模板 DSL，同时给完全自定义字段留下空间。

选择这个方案的原因：

- 它解决的是 setup 的根问题：字段定义终于成为可浏览、可编辑的结构，而不是一串 CSV。
- 保留“字段 ID 就是模板占位符名”的规则后，用户不需要再学习额外模板语法。
- 兼容默认字段 ID 后，现有模板和 CLI 参数仍可以继续工作。

备选方案：

- 继续保留 `annotate.form.fields = ["reason", ...]` 与 `annotate.reference_kinds = [...]`：无法在 setup 中表达标签、字段类型和子配置。
- 一步上马任意动态表单/校验 DSL：灵活度更高，但超出当前需要。

### 决策 5：旧配置按“读取兼容、保存规范化”的策略迁移

兼容策略分两层：

- **读取阶段**：继续接受旧的 `annotate.form.fields = ["reason", ...]` 与 `annotate.reference_kinds = [...]`，在内存中转换为结构化字段定义
- **保存阶段**：一旦用户通过新 setup 保存配置，统一写回新的结构化模型

对 feature toggles 也采用同样思路：

- 旧配置缺少 `reset` / `checkout_remote` / `completion` 时，默认视为 `true`
- setup 保存后写入完整 feature 集

选择这个方案的原因：

- 用户无需先手工改配置文件才能进入新 setup。
- 代码内部只需要面向一个规范化后的运行时模型工作。

备选方案：

- 要求用户先手工迁移配置：最省实现成本，但会显著伤害可用性。
- 永久支持旧新两套写回格式：长期维护成本更高。

### 决策 6：注释设置按任务域拆成稳定子菜单

`annotate` 不再是一页 20 个字段，而是至少拆成以下子菜单：

- `引用与表单`
- `代码文件类型`
- `渲染行为`
- `新增格式`
- `修改格式`
- `删除格式`
- `旧代码处理`

其中：

- `新增/修改/删除格式` 内部各自包含“启用自定义格式”开关与起始/结束模板编辑入口
- `旧代码处理` 继续暴露结构化旧代码展示策略
- `引用与表单` 负责字段定义和 option set 的维护

选择这个方案的原因：

- 用户表达的是按任务组织配置，而不是按存储字段组织配置。
- 这能把每一层的列表长度控制在可浏览范围内，避免再次出现“一页 20 项”的问题。

备选方案：

- 只把当前字段原样搬进树里：结构上有层级了，但心智模型仍然混乱。

## 风险 / 权衡

- [菜单树状态机会比当前索引表单更复杂] → 通过统一节点类型和路径栈，把复杂度集中在一套模型里，而不是分散在多个索引函数中。
- [结构化字段定义会扩大 annotate 配置模型] → 先限制为 `text` 与 `single-select` 两类标量字段，避免一次做成通用表单引擎。
- [旧配置自动规范化后，首次保存会改变 TOML 结构] → 通过“读取兼容、保存规范化”明确这一行为，并在发布说明与测试中覆盖。
- [更多 feature gate 会增加命令帮助与执行路径的判断分支] → 统一复用现有 `push` / `annotate` 的禁用态处理模式，避免每个命令各写一套逻辑。
- [单栏布局取消左侧总览后，用户可能失去全局位置感] → 用 breadcrumb、页面标题和稳定的 `Esc` 返回语义补足。

## Migration Plan

1. 先引入新的 setup 菜单节点模型，并在不改配置语义的前提下完成界面单栏化。
2. 扩展 `FeaturesConfig`，让 `reset`、`checkout_remote`、`completion` 默认启用并接入命令守卫与 help 展示。
3. 引入 annotate 结构化字段定义模型，并在加载阶段兼容旧 `fields + reference_kinds` 配置。
4. 把注释设置重组为子菜单；确认 setup 保存时写回新的规范化结构。
5. 为 setup 导航状态机、配置兼容迁移、feature gate 执行与 annotate 字段解析补充回归测试。

回滚策略：

- 如果结构化字段定义实现过大，可以先保留新 setup 导航与 feature gate 扩展，把 annotate 字段定义的落盘切换拆到后续小变更。
- 如果单栏布局在终端兼容性上暴露问题，可以保留树节点状态机，但暂时用更保守的列表渲染样式。

## Open Questions

- 自定义字段的 `label` 是否需要区分中英文，还是当前阶段先按配置值原样显示？
- `choice` 类型字段在 setup 中是直接 `Enter` 循环切换，还是进入一个标准子菜单再选择候选值？当前设计倾向于统一进入子菜单，以减少特殊交互。
- CLI 参数是否需要在后续支持为自定义字段赋值，例如 `xgit annotate --field ticket=ABC-1`？本次先不纳入范围，但模型需要为未来保留空间。
