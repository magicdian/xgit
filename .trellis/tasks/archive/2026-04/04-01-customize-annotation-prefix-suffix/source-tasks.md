## 1. 配置模型与兼容归一化

- [x] 1.1 在 `xgit/src/config.rs` 和默认配置中新增 `annotate.block_templates` 结构，为 `add`、`modify`、`del` 提供 `start`/`end` 默认模板。
- [x] 1.2 更新配置加载与合并逻辑，实现“新 `block_templates` 优先、旧 `annotate.policies.*` 回退”的归一化行为，并补充对应单元测试。

## 2. 注释渲染链路

- [x] 2.1 改造 `xgit/src/annotate.rs` 的 `c_line_block` 渲染逻辑，使其使用配置中的起始模板和结束模板生成注释边界，不再硬编码 `// end <kind>`。
- [x] 2.2 保持 `modify` / `del` 的旧代码记录兼容语义，并增加覆盖自定义结束模板（例如 `//@}`）和旧配置回退行为的渲染测试。

## 3. 设置界面与回归验证

- [x] 3.1 扩展 `xgit setup` 的注释策略栏目，支持编辑 `add`、`modify`、`del` 各自的起始模板与结束模板，并在加载旧配置时展示归一化后的字段值。
- [x] 3.2 更新相关文案、保存逻辑和回归测试，验证 setup 保存后的配置与 `xgit annotate` 渲染结果都符合新的前后缀模板模型。
