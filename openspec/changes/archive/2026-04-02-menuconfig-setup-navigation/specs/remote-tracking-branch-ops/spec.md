## 新增需求

### 需求:xgit reset 与 xgit checkout-remote 必须遵循功能开关状态
`xgit reset` 与 `xgit checkout-remote` 必须在执行前检查有效配置中的功能开关；当对应功能被关闭时，系统必须拒绝执行，而不是继续进入 Git 远端解析、upstream 校验或 checkout/reset 逻辑。

#### 场景:reset 功能关闭时拒绝执行
- **当** 有效配置将 `reset` 功能设为关闭
- **当** 用户执行 `xgit reset`
- **那么** 系统必须拒绝执行
- **并且** 系统必须提示该功能当前已被配置关闭

#### 场景:checkout-remote 功能关闭时拒绝执行
- **当** 有效配置将 `checkout-remote` 功能设为关闭
- **当** 用户执行 `xgit checkout-remote 8676_os6_xpdev`
- **那么** 系统必须拒绝执行
- **并且** 系统必须提示该功能当前已被配置关闭

## 修改需求
<!-- 无 -->

## 移除需求
<!-- 无 -->
