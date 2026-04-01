# 提案：xgit — 自动识别 remote 并推送（单仓库）

## 什么（概要）
实现一个跨平台命令行工具 `xgit`，用于在当前目录对应的单个 Git 仓库中自动识别应使用的 remote，并将当前分支或指定分支推送到该 remote。支持 Gerrit 推送（`refs/for/<branch>`）以及 `--no-thin` 选项以避免 thin-pack 导致无法推送的情况。

## 为什么（动机）
- 在 AOSP / repo 场景中，同一仓库可能出现不同命名但等价的 remote（如 `origin`、`origin2`），手动分辨繁琐且容易出错。
- Gerrit 推送需要 `refs/for/*` 语义，且不同仓库的 remote 名称并不一致。
- 某些服务器或网络条件下，thin-pack 会导致推送失败；需要 `--no-thin` 支持确保完整推送。

## 目标用户与场景
- 开发者在多仓库（尤其是 AOSP 源树）中工作，但每次只需对当前子仓库操作。
- CI 或脚本化场景下需要确定一个非交互、可预测的推送结果。

## 成功标准
- 在常见仓库配置下，`xgit push` 能自动选择正确的 remote 并成功推送（包含 Gerrit 场景）。
- 支持 `xgit push <branch>`、`--remote`、`--no-thin`、`--dry-run` 等选项。
- 在无法自动决策时提供明确错误信息并建议使用 `--remote`。

## 非目标（暂不实现）
- 批量遍历 top-level repo（`repo` 多仓库操作）——可作为后续扩展。
- 深度集成 Gerrit push-option（初期仅支持基本 refs/for 语义）。
