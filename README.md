# SnipChat · 截屏助手

Windows 截屏工具：区域框选 / 全屏 → 标注（框/箭头/文字/马赛克）→ 自动进剪贴板 + 落盘 PNG → 一键把截图和你的问题发给本机 [codex](https://github.com/openai/codex) CLI，多轮追问、中途加图继续聊。

## 快捷键

| 键 | 动作 |
|---|---|
| Ctrl+Alt+A | 区域截屏（拖拽框选，放大镜像素级对齐，Enter/双击确认，Esc 取消） |
| Ctrl+Alt+S | 全屏截屏（直接进标注编辑器） |

托盘常驻：区域截屏 / 全屏截屏 / 打开主面板 / 退出。主窗关闭 = 收进托盘。

## 截图去向

- 自动复制到剪贴板（可直接粘贴）
- 自动保存到 `图片\SnipChat\截图_YYYYMMDD_HHMMSS.png`

## 问 codex（可选功能）

需要本机已安装并登录 codex CLI：

```bash
npm i -g @openai/codex
codex
```

用法：截图完成时自动挂到主窗输入框 → 输入问题回车。首轮 `codex exec -i 图 问题`，追问自动走 `codex exec resume --last` 续聊；续聊带图失败时自动开新线程并携带最近对话摘要，不丢上下文。

## 构建 exe

仓库自带 GitHub Actions：打 tag 自动产出 NSIS 安装包 + 绿色 exe，挂在 Releases。

本地构建：

```bash
# 需要 Rust stable + Windows
cd src-tauri
cargo tauri build
# 产物：src-tauri/target/release/snipchat.exe
```

仅 Windows（截屏走 Win32 GDI）。首版范围：主显示器。

## 目录

```
ui/                 前端（纯 HTML/JS，无构建步骤）
  index.html/app.js 主窗：codex 聊天 + 图片附件
  overlay.html/js   框选浮层：拖拽选区 + 放大镜
  editor.html/js    标注编辑器：框/箭头/文字/马赛克/撤销
src-tauri/src/
  main.rs           托盘/热键/窗口管理
  capture.rs        GDI 截屏、落盘、剪贴板
  codex.rs          codex CLI 子进程 + 流式回传
.github/workflows/release.yml   打包发布
```
