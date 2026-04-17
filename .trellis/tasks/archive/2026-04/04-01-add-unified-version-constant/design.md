> Imported historical task context normalized by Transpec for Trellis-first continuation. Use `.trellis/spec/` as the current source of truth; preserved source artifacts remain for provenance.

## 上下文

当前项目存在 `xgit/Cargo.toml`，其中 `package.version` 已天然承载版本语义，但代码侧尚未明确约束“版本展示必须来自 Cargo 元数据”。这会导致未来出现 CLI 显示版本与构建版本不一致的问题。

用户希望统一版本维护入口，并认可“关联 Cargo 版本”是更优路径；版本表达采用 `YYMM.DD.BuildNumber` 更直观，但落地仍需满足 Cargo 语法约束。因此本设计以 Cargo 版本为单一来源，格式约束以 Cargo 可接受范围为先。
本变更最终采用目标语义 `2604.01.2`，并落地为 Cargo 兼容写法 `2604.1.2`。

## 目标 / 非目标

**目标：**
- 提供唯一的版本号来源：`Cargo.toml` 的 `package.version`。
- 所有版本信息展示与引用统一走 Cargo 版本访问入口，消除多处硬编码。
- 提供清晰的版本查询入口（CLI 层）。

**非目标：**
- 不自动生成版本号（不接入日期自动计算或 CI 自动写入）。
- 不在本次变更中重构与版本无关的 CLI/i18n 架构。
- 不引入复杂的发布流水线改造（如自动计算并写回 Cargo 版本）。

## 决策

### 决策 1：以 Cargo package.version 作为唯一版本来源
- 方案：运行时版本统一通过 `env!("CARGO_PKG_VERSION")`（或 `clap` 的 `crate_version!()`）读取，不再维护独立手工版本常量。
- 理由：`Cargo.toml` 已是 Rust 项目标准版本源，复用它能避免“双版本源”漂移。
- 备选：
  - 新增独立 `APP_VERSION` 常量并手工维护：会与 Cargo 版本形成双写风险。
  - 多处按需读取 `Cargo.toml` 文件：实现复杂且运行时文件解析不必要。

### 决策 2：CLI 版本展示统一绑定 Cargo 版本访问入口
- 方案：在 CLI 命令构建时显式绑定从 Cargo 元数据获取的版本值，并保证 `xgit --version` 输出该值。
- 理由：版本查询是“增加版本号信息”的最直接用户触点，且易于自动化测试。
- 备选：
  - 只在 help 文本中写死版本：仍属于硬编码，且不利于标准化查询。
  - 仅在日志打印版本：可见性弱，不符合 CLI 常规体验。

### 决策 3：通过代码扫描与测试约束防止回归硬编码
- 方案：补充测试（或检查脚本）校验版本入口输出，并限制业务代码直接出现版本字面量。
- 理由：仅靠约定不足以长期约束，需最小自动化保障。
- 备选：
  - 仅在 code review 口头约束：执行成本高且容易漏。

## 风险 / 权衡

- [风险] `YYMM.DD.BuildNumber` 的 `DD` 可能包含前导零，不完全符合 Cargo 版本规范  
  → 缓解措施：采用等价的 Cargo 兼容写法（如 `2604.1.2`）并在文档中保留语义映射。

- [风险] 后续新模块再次硬编码版本字符串  
  → 缓解措施：在任务中加入替换与扫描步骤，并添加回归测试。

- [风险] 版本格式在团队内缺少统一约定  
  → 缓解措施：在文档中给出建议格式与示例，并在发布流程中进行人工检查。

## Migration Plan

1. 更新 `Cargo.toml` 的 `package.version` 为 `2604.1.2`。  
2. CLI 版本查询入口改为统一读取 Cargo 版本。  
3. 搜索并替换现有版本号字面量引用为统一版本访问入口。  
4. 增加测试覆盖（版本输出一致性与硬编码回归检查）。  
5. 回滚策略：若出现兼容性问题，可临时恢复旧展示方式，但保持 Cargo 作为版本源不变。

## Open Questions

- 是否需要在 `xgit help` 首页额外展示版本号，还是仅保留 `--version` 入口？
