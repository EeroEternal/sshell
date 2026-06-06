# sshell (Structured Shell) Architecture Design

## 1. 愿景与目标 (Vision & Goals)

`sshell` 是专为 **人机无缝协同编程 (Human-Agent Pair Programming)** 打造的新一代终端抽象层。传统的 PTY 终端在面对现代 LLM 时暴露出诸如上下文截断、折行破环数据结构、异步轮询困难以及人机抢占等致命缺陷。`sshell` 的目标是提供一套“语义化、结构化、双轨制”的终端中间件，为 AI Agent 提供精准、可靠的交互接口，同时保留人类开发者的原始彩色界面体验。

本项目深受 Zed IDE 的架构启发，并充分复用 Zed 在终端 PTY 底层和高性能 Rust 生态上的工程经验（如 `portable-pty` 等库）。

## 2. 核心架构 (Core Architecture)

`sshell` 的整体架构由以下五个核心支柱构成：

### 2.1 双轨捕获机制 (Dual-Track Capture)
打破大模型只看一块 2D 截屏的历史，实现数据流的分离：
- **Human Track (人眼轨)**: 继续挂载底层的 PTY，保留所有 ANSI 转义序列、颜色、重绘逻辑（如进度条），并通过 GUI/CLI 终端展现给人类。
- **Agent Track (机器轨)**: 旁路收集完整的、去格式化的原始输出（Shadow Stream）。对于长日志构建任务，AI Agent 可通过结构化接口分页获取完整的底层流，彻底消除 PTY 滚动条丢失 (Scrollback Loss) 和屏幕宽度导致的折行破坏。

### 2.2 语义化 Shell 协议 (Semantic Shell Protocol)
告别基于轮询（Polling）和超时的终端状态判断，引入事件驱动模型（Event-Driven Model）。
- 通过底层 Shell 集成或基于底层的探针，向系统发出明确的生命周期事件：
  - `CommandStart(cmd)`
  - `CommandEnd(exit_code)`
  - `PromptWaiting()`
- Agent 不再需要依靠大模型猜测命令是否跑完，而是通过监听事件总线直接获取状态，根除 Race Condition。

### 2.3 API 优先的工具适配层 (API-First Adapters)
并不是所有的工作都需要通过终端 UI 完成。
- `sshell` 内置一个拦截与重写引擎（Middleware）。当 Agent 尝试执行如 `cargo build` 或 `npm install` 时，`sshell` 会自动尝试追加 `--message-format=json` 或等效的结构化输出参数。
- Agent 获取到的是纯粹的、附带行列号的 JSON AST，从而大幅降低 LLM 提取错误信息的难度和 Token 消耗。

### 2.4 终端状态 DOM 化 (Terminal DOM)
在必须使用 TUI（文本用户界面）的场景下，摒弃直接拉取纯文本屏幕的做法。
- 解析 PTY 中的颜色和光标信息，生成具有语义属性的“伪 DOM 树”。
- 例如：将终端的输出解析为带有 `<highlight>` 或 `<error>` 标签的结构，让大模型能“看见”错误高亮，精准定位被选中的交互菜单项。

### 2.5 人机输入仲裁锁 (Input Contention Mutex)
在终端状态机中引入明确的 **所有权 (Ownership)** 机制：
- 区分 `HumanDriving` 和 `AgentDriving`。
- 当 Agent 正在接管终端（如回答 TUI 的多步问答）时，开启锁定机制，阻断人类误触键盘的信号；当 Agent 死锁时，人类可通过显式的“打断 (Take Over)”指令抢回所有权，避免输入相互交织 (Interleaving) 引发的混乱。

## 3. 技术选型与实现路线 (Tech Stack)

充分复用 Zed 积累的 Rust 终端生态技术：
- **PTY 管理**: `portable-pty` 用于跨平台的伪终端创建与基础进程管理。
- **终端状态模拟**: 复用/参考 `alacritty_terminal` 的 VTE（Virtual Terminal Emulator）解析器逻辑，用于构建 Terminal DOM 树。
- **并发与异步**: `tokio` 驱动全局事件循环，支持高并发的事件派发机制。
- **序列化通信**: `serde` 和 `serde_json`，对外提供供 Python/TypeScript 写的 LLM Client 调用标准 JSON-RPC 或 WebSocket 接口。

## 4. 阶段规划 (Roadmap)

1. **Phase 1: 核心通道与双轨制** - 搭建底层 `portable-pty`，实现对命令 stdout/stderr 的旁路收集，暴露 `TerminalState`。
2. **Phase 2: 语义化与事件派发** - 集成基础的 Shell Integration，实现对命令启停状态的准确事件通知。
3. **Phase 3: 终端 DOM 解析与 API 适配** - 增加针对 LLM 优化的内容提取函数，实现 `get_semantic_content()`。
4. **Phase 4: IPC/RPC 封装** - 提供跨语言可用的 Agent Client Protocol，使其能被集成到任意 AI IDE（包括 Zed 本身的插件体系）。
