# sshell (Structured Shell)

`sshell` 是一款专为 **人机无缝协同编程 (Human-Agent Pair Programming)** 打造的新一代终端中间件。
它基于 `portable-pty`，通过双轨捕获 (Dual-Track Capture)、API 拦截器和语义化 Hook，完美解决了传统 AI Agent 读取终端输出时面临的折行截断、状态竞态和滚屏丢失等痛点。

## 特性 (Features)
- **双轨捕获机制 (Dual-Track Capture)**: 旁路收集完整的原始流，解决传统 Agent PTY 截屏造成的长文本丢失。
- **语义化 Shell (Semantic Hooks)**: 通过捕获底层 OSC 133 等逃逸序列，实现精准的事件驱动 (`CommandStart`, `CommandEnd`)，摒弃传统轮询猜测。
- **API 优先适配器 (Adapters)**: 自动对 `cargo build`、`npm install` 等高频开发命令追加结构化输出参数（如 JSON 格式），提升大模型读取精确度。
- **人机输入仲裁锁 (Input Mutex)**: 支持显式声明 `HumanDriving` 与 `AgentDriving` 状态，彻底避免人类与 Agent 输入交织造成的死锁与错乱。

## 快速入门 (Quick Start)

以下代码展示了如何在一个 Rust 编写的 AI Agent 中使用 `sshell` 来派生、控制并监听一个进程。

### 1. 引入依赖
在你的新项目中：
```toml
[dependencies]
sshell = { git = "git@github.com:EeroEternal/sshell.git" }
tokio = { version = "1", features = ["full"] }
```

### 2. 使用示例 (Usage Example)

```rust
use sshell::{StructuredShell, ShellEvent, TerminalOwnership};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 初始化 sshell 引擎
    let shell = StructuredShell::new();

    // 2. 订阅终端的生命周期事件（大模型的“耳朵”）
    let mut event_receiver = shell.subscribe();

    // 3. 锁定控制权（代表此时是 Agent 在操作，避免人类误触终端）
    shell.lock_for_agent();

    // 4. 执行命令。
    // 注意：sshell 内置的 Adapter 会自动将其转换为 `cargo build --message-format=json`
    println!(">>> Agent 正在启动构建任务...");
    let mut session = shell.spawn_pty("cargo build")?;

    // 5. 监听双轨异步事件
    while let Ok(event) = event_receiver.recv().await {
        match event {
            ShellEvent::CommandStart(cmd) => {
                println!("[Hook] 命令已精准启动: {}", cmd);
            }
            ShellEvent::OutputChunk(chunk) => {
                // 这是无截断、无格式损失的纯净旁路数据，Agent 可以放心用于 JSON 解析
                println!("[Data] 截获旁路输出: {} 字节", chunk.len());
            }
            ShellEvent::WaitingForInput => {
                println!("[Hook] 程序挂起，正在等待人类或 Agent 的交互输入...");
                // 在这里 Agent 可以调用 session.write_input("y\n", true) 来自动确认
            }
            ShellEvent::CommandEnd(exit_code) => {
                println!("[Hook] 命令执行完毕，退出码: {}", exit_code);
                break; // 任务结束，跳出循环
            }
        }
    }

    // 6. 任务完成后，交还终端控制权给人类
    shell.release_to_human();
    println!(">>> Agent 操作完毕，控制权已交还。");

    Ok(())
}
```
