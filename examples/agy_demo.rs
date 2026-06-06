use anyhow::Result;
use sshell::{ShellEvent, StructuredShell};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    println!("===================================================");
    println!("🤖 [Agy Agent]: 正在唤醒...");
    println!("🤖 [Agy Agent]: 正在初始化 sshell 双轨语义化终端...");
    println!("===================================================\n");
    
    let shell = StructuredShell::new();
    let mut event_receiver = shell.subscribe();

    println!("🤖 [Agy Agent]: 开启人机输入互斥锁 (锁定终端为 Agent 控制模式)...");
    shell.lock_for_agent();

    // 假设 Agy Agent 要执行一个有耗时和输出的命令
    // 因为这里我们没有真实的带结构化支持的 cargo，我们用一个 echo 和 sleep 来模拟长命令
    // sshell 的 adapters 模块会对 cargo 有反应，你可以把它替换为真实项目的构建指令体验
    let cmd = "echo 'Running analysis...' && sleep 2 && echo '\x1b[31m[Error] File not found\x1b[0m' && sleep 1";
    
    println!("🤖 [Agy Agent]: 决策完毕，准备派生命令: `{}`", cmd);
    let mut _session = shell.spawn_pty(cmd)?;

    let mut captured_output = String::new();

    println!("🤖 [Agy Agent]: 进入睡眠/不轮询状态，完全依靠 Semantic Shell 事件总线驱动...\n");
    
    // 大模型通过事件循环等待结果，而非传统的轮询死循环
    while let Ok(event) = event_receiver.recv().await {
        match event {
            ShellEvent::CommandStart(cmd) => {
                println!(">> [Hook 事件抛出] 检测到底层进程启动: {}", cmd);
            }
            ShellEvent::OutputChunk(chunk) => {
                // Agent 正在后台疯狂收集没有截断的纯粹原始旁路数据
                captured_output.push_str(&chunk);
                println!("   (双轨捕获: 旁路流抓取到 {} 字节数据)", chunk.len());
            }
            ShellEvent::WaitingForInput => {
                println!(">> [Hook 事件抛出] 程序挂起，正在等待交互式输入...");
                // 如果是需要交互的程序，Agent 可以通过 _session.write_input("y\n", true) 来交互
            }
            ShellEvent::CommandEnd(exit_code) => {
                println!(">> [Hook 事件抛出] 进程彻底结束，收到退出码: {}", exit_code);
                break;
            }
        }
    }

    println!("\n🤖 [Agy Agent]: 终端任务已完成！");
    println!("🤖 [Agy Agent]: 让我来看看我在后台【机器轨 (Shadow Track)】里完整捕获到了什么数据：");
    println!("--------------------------------------------------");
    // 这里的输出对于人类可能带有乱码（终端控制符），但这就是 Agent 最爱的结构化或原生的输出
    println!("{:?}", captured_output.trim());
    println!("--------------------------------------------------");

    println!("🤖 [Agy Agent]: 任务彻底完成，正在释放终端互斥锁...");
    shell.release_to_human();
    println!("🤖 [Agy Agent]: 控制权已交还人类。Agy 退下。");

    Ok(())
}
