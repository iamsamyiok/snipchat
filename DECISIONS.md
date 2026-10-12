# 决策日志（契约 R3）

## D1 技术栈 Rust + Tauri 2（用户选定）
体积目标 5-10MB。前端纯静态 HTML/JS，无 Node 构建链——CI 更简单，出错面更小。

## D2 截屏 Win32 GDI 而非第三方 crate
CreateCompatibleDC + BitBlt(SRCCOPY|CAPTUREBLT) + GetDIBits 取 32bpp，BGRA→RGBA 后用 image crate 编 PNG。不引 dxgi/ffmpeg 系重依赖。v1 仅主显示器，多屏进 backlog。

## D3 框选在 JS 层完成裁剪
Rust 抓整屏存全局 → overlay 窗口（透明全屏置顶）展示，canvas 裁剪后 `handoff_crop` 回传 Rust 暂存，编辑器取用。物理/CSS 像素换算用 `devicePixelRatio`，Rust 侧不碰 DPI API。

## D4 codex 集成走子进程而非 API
调用用户本机 codex CLI（复用其登录态与计费）。首轮 `codex exec --skip-git-repo-check -i <img> <prompt>`；追问 `exec resume --last`；resume 带 `-i` 失败自动降级：新线程 + 前端镜像来的最近 8 轮摘要（codex_mirror_transcript）。stdout 逐行 `codex:delta` 事件流式渲染。

## D5 剪贴板用 arboard（Rust 侧）
静态前端拿不到插件 JS API（无打包器），arboard 在主线程写图最稳。

## D6 exe 分发：GitHub Actions tag 构建
沙箱是 Linux，无法产 Windows exe（对齐轮红色盲区已确认）。`release.yml`：windows-latest + rust stable + tauri-action，tag `v*` 触发，NSIS + 绿色 exe 上 Release。push 到 main 也跑一次构建当编译验证。

## D7 时区/命名
chrono::Local 格式化 `截图_%Y%m%d_%H%M%S.png`，目录 `图片\SnipChat`（避开系统 Win+PrintScreen 的 Screenshots 目录）。
