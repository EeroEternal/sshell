use anyhow::Result;
use rustyline::DefaultEditor;
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

    // 修改交互脚本：这次我们要求输入一个具体的动作（比如：执行），而不是简单的 Y/N
    let cmd = "echo 'Starting critical operation...' && sleep 1 && echo -n 'Please enter action to perform (e.g., 执行): ' && read ans && echo '' && echo \"[Script] Received your command: $ans\" && sleep 1";
    
    println!("🤖 [Agy Agent]: 决策完毕，准备派生高危交互命令: `{}`\n", cmd);
    let mut session = shell.spawn_pty(cmd)?;

    let mut captured_output = String::new();

    // 持续监听底层事件
    while let Ok(event) = event_receiver.recv().await {
        match event {
            ShellEvent::CommandStart(_) => {}
            ShellEvent::OutputChunk(chunk) => {
                captured_output.push_str(&chunk);
                // 同步回显到屏幕上给人类看
                print!("{}", chunk);
                io::stdout().flush()?;

                // 启发式检测：发现终端等待具体指令
                if chunk.contains("Please enter action to perform") {
                    println!("\n\n🤖 [Agy Agent]: 🚨 警告！程序挂起，需要人类提供具体的执行指令！");
                    println!("🤖 [Agy Agent]: 正在释放终端互斥锁 (Release lock) ...");
                    shell.release_to_human();
                    
                    // 【关键修复】：使用 rustyline 库来提供强大的终端行编辑器能力
                    // 它完美支持中文输入、退格键(Backspace)、方向键等，彻底拒绝逃逸乱码！
                    let mut rl = DefaultEditor::new()?;
                    let readline = rl.readline("👨‍💻 [人类 (你)]: 请自由输入内容 (如输入 '执行') 并按回车: ");
                    
                    match readline {
                        Ok(line) => {
                            // 将带有换行符的完整输入灌入底层 PTY
                            session.write_input(&format!("{}\n", line), false)?;
                        },
                        Err(_) => {
                            println!("输入取消。");
                            break;
                        }
                    }
                    
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
