## 1. setup 单栏树形导航骨架

- [x] 1.1 在 `xgit/src/setup_ui.rs` 中用统一的菜单节点/菜单帧模型替换当前 `(section, field)` 双栏状态机，支持 breadcrumb、逐级进入与逐级返回
- [x] 1.2 将 setup 根菜单重组为“通用设置 / 身份设置 / 注释设置”等层级入口，并统一 `Up/Down`、`Enter`、`Space`、`Esc` 的交互语义
- [x] 1.3 把“代码文件类型”从特例弹窗改造成标准层级化多选子菜单，支持分类层与扩展名层之间的进入、返回和勾选切换
- [x] 1.4 更新 setup 相关状态机测试与界面文案，覆盖根菜单展示、子菜单导航、ESC 返回和脏状态退出确认

## 2. 功能开关与命令可用性

- [x] 2.1 在 `xgit/src/config.rs` 中扩展 feature toggle 模型与默认值，纳入 `reset`、`checkout-remote` 与 `completion`，并保持旧配置缺省时向后兼容
- [x] 2.2 在 setup 的“功能开关”菜单中暴露 `push`、`annotate`、`reset`、`checkout-remote` 与 `completion` 的启用状态编辑入口
- [x] 2.3 在 `xgit/src/main.rs`、相关帮助输出和命令执行链路中接入新的功能开关校验，确保 `help` 与 `setup` 始终可用
- [x] 2.4 为 `push`、`annotate`、`reset`、`checkout-remote` 与 `completion` 的禁用路径补充自动化验证

## 3. 注释表单结构化与兼容迁移

- [x] 3.1 在 `xgit/src/config.rs` 中引入结构化的 annotate 表单字段定义与 option set 模型，并将旧版 `annotate.form.fields` / `annotate.reference_kinds` 兼容映射为运行时规范化结构
- [x] 3.2 在 setup 中新增“引用与表单”子菜单，允许浏览和编辑字段定义、引用类型集合及其摘要，而不是直接编辑 CSV
- [x] 3.3 在 `xgit/src/annotate.rs` 中按结构化字段定义采集运行时上下文，并让模板按字段 `id` 展开占位符，同时保留 `reason`、`reference_kind`、`reference_value` 的兼容行为
- [x] 3.4 为旧配置兼容读取、自定义字段采集和自定义占位符展开补充回归测试

## 4. 注释设置菜单重组

- [x] 4.1 将“注释设置”拆分为“引用与表单 / 代码文件类型 / 渲染行为 / 新增格式 / 修改格式 / 删除格式 / 旧代码处理”等稳定子菜单
- [x] 4.2 在 `add` / `modify` / `del` 三类格式子菜单中实现“启用自定义格式”开关，以及起始模板/结束模板的逐级编辑入口
- [x] 4.3 在“渲染行为”和“旧代码处理”子菜单中继续承载现有缩进对齐、空白行包裹、日期格式和旧代码展示策略等配置项，并更新对应摘要显示

## 5. 文案、文档与收尾验证

- [x] 5.1 更新 `xgit/resources/i18n/zh-CN.toml` 与 `xgit/resources/i18n/en-US.toml`，补充新的菜单节点、路径标题、状态提示和功能禁用文案
- [x] 5.2 更新 `xgit/README.md` 与相关帮助说明，描述新的 menuconfig 式 setup 导航、功能开关和结构化注释字段配置方式
- [x] 5.3 运行相关测试并修正回归问题，确保 setup、config、annotate 与命令开关行为符合新规格
