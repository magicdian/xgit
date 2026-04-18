> Imported historical task context normalized by Transpec for Trellis-first continuation. Use `.trellis/spec/` as the current source of truth; preserved source artifacts remain for provenance.

## 为什么

当前 `xgit annotate --latest-commit` 的结束提示会输出类似“注释渲染完成：10/11”，但这个分母同时包含了成功渲染和被跳过的文件，用户只能从前面的零散“跳过”日志里自行拼凑原因，无法在命令结束时快速判断哪些文件其实没有被格式化。更关键的是，像 `Android.bp` 这类“系统根本不支持的类型”和“`.cpp` 已知但当前设置没开启”的场景现在都会落成同一种“未匹配到文件规则”，这会让用户不知道该去调整 setup，还是接受该类型暂不支持。

## 变更内容

- 调整 annotate 的结果汇总逻辑，把“已渲染文件”和“未格式化文件”分开呈现；当存在未处理文件时，在结束阶段统一输出清单，而不是只依赖执行过程中的零散跳过日志。
- 为未格式化文件增加稳定、可解释的原因分类，至少区分：
  - 系统当前不支持该类型，例如 `Android.bp`
  - 系统已知该后缀，但当前设置未开启该后缀功能，例如用户关闭了 `*.cpp`
  - 命中了规则但对应渲染器尚未实现
- 统一未处理文件的终端展示格式，使用户在 latest-commit 和 staged 模式下都能直接看到类似 `未格式化：<path> (<reason>)` 的结论。
- 为新的汇总与原因分类补充回归测试，覆盖“已知但被禁用的后缀”和“完全未知/不支持类型”两类核心场景。
- 将本次发版目标同步更新为 `2604.1.4`，并继续要求版本信息统一来源于 `Cargo.toml` 的 `package.version`。

## 功能 (Capabilities)

### 新增功能
<!-- 无 -->

### 修改功能
- `annotation-normalization`: annotate 在处理候选文件后，需要把未格式化文件作为显式结果输出，并给出与配置状态一致的跳过原因，而不是只输出模糊的总数和单一“未匹配到文件规则”提示。
- `versioning-source-unification`: 本次版本目标需要更新为 `2604.1.4`，并继续要求 CLI 与代码实现统一从 `Cargo.toml` 的 `package.version` 读取版本值。

## 影响

- `xgit/src/annotate.rs` 需要把当前“直接打印跳过日志 + 最后输出 `count/total`”的流程改造成带有结构化结果汇总的输出模型。
- `xgit/src/code_file_types.rs` 需要作为“系统是否已知该后缀”的判断来源，支持 annotate 在运行时区分“未支持类型”和“设置未开启”。
- `xgit/resources/i18n/zh-CN.toml` 与 `xgit/resources/i18n/en-US.toml` 需要补充新的未格式化原因与汇总文案。
- annotate 相关测试需要扩充，验证 `Android.bp`、被禁用的 `*.cpp` 以及未实现渲染器三类跳过原因的输出行为。
- `xgit/Cargo.toml` 与版本相关测试/文档需要同步更新到 `2604.1.4`，确保 `xgit --version` 继续与 Cargo 版本来源保持一致。
