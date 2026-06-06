use anyhow::Result;
use sshell::{ShellEvent, StructuredShell};
use std::io::{self, Write};

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

    // 使用一个会挂起并等待交互的 bash 脚本
    let cmd = "echo 'Starting critical operation...' && sleep 1 && echo -n 'Are you sure you want to deploy? [y/N]: ' && read ans && echo '' && echo \"[Script] Received your answer: $ans\" && sleep 1";
    
    println!("🤖 [Agy Agent]: 决策完毕，准备派生高危交互命令: `{}`\n", cmd);
    let mut session = shell.spawn_pty(cmd)?;

    let mut captured_output = String::new();

    // 持续监听底层事件
    while let Ok(event) = event_receiver.recv().await {
        match event {
            ShellEvent::CommandStart(cmd) => {
                // Ignore for now
            }
            ShellEvent::OutputChunk(chunk) => {
                captured_output.push_str(&chunk);
                // 同步回显到屏幕上给人类看
                print!("{}", chunk);
                io::stdout().flush()?;

                // 启发式检测：Agent 发现终端界面里弹出了 [y/N] 的交互提示
                if chunk.contains("[y/N]:") {
                    println!("\n\n🤖 [Agy Agent]: 🚨 警告！程序挂起，需要人类确认决策！");
                    println!("🤖 [Agy Agent]: 正在释放终端互斥锁 (Release lock) ...");
                    shell.release_to_human();
                    
                    // 读取真实人类用户的键盘输入
                    print!("👨‍💻 [人类 (你)]: 请在键盘上输入你的回答 (例如 y) 并按回车: ");
                    io::stdout().flush()?;
                    
                    let mut input = String::new();
                    // 这里为了简单的 Demo 效果直接阻塞读取标准输入
                    io::stdin().read_line(&mut input)?;
                    
                    // 模拟人类开发者向 PTY 终端灌入输入（由于之前已 release_to_human，此时写入合法）
                    session.write_input(&input, false)?;
                    
                    println!("🤖 [Agy Agent]: 人类输入完毕！重新开启输入互斥锁 (Lock)，Agent 接管剩余流程！\n");
                    shell.lock_for_agent();
                }
            }
            ShellEvent::WaitingForInput => {}
            ShellEvent::CommandEnd(exit_code) => {
                println!("\n>> [Hook 事件抛出] 进程彻底结束，收到退出码: {}", exit_code);
                break;
            }
        }
    }

    println!("\n🤖 [Agy Agent]: 终端任务已完成！");
    println!("🤖 [Agy Agent]: 正在释放终端互斥锁...");
    shell.release_to_human();
    println!("🤖 [Agy Agent]: 控制权已交还人类。Agy 退下。");

    Ok(())
}
