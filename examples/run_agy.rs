use anyhow::Result;
use rustyline::DefaultEditor;
use sshell::{ShellEvent, StructuredShell};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    let shell = StructuredShell::new();
    let mut event_receiver = shell.subscribe();

    println!("===================================================");
    println!("🚀 [sshell 代理]: 准备挂载本机实际安装的 `agy` 进程...");
    println!("说明：这个 Demo 证明了 sshell 完全可以包装一个复杂的交互式 Agent CLI 工具。");
    println!("===================================================\n");

    // 确保终端互斥锁放开给人类
    shell.release_to_human();

    // 真正派生本机环境中的 agy 程序！
    let mut session = shell.spawn_pty("agy")?;

    // 创建一个跨线程通道，把人类键盘输入发回给主线程
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);

    // 开启单独的输入监听线程，避免阻塞异步的输出接收流
    std::thread::spawn(move || {
        // 由于 agy 是交互式的，我们同样套一层 rustyline 防止退格键乱码
        let mut rl = DefaultEditor::new().unwrap();
        loop {
            // 注意这里提示符为空，因为真正的 agy 会自己打印自己的提示符！
            match rl.readline("") {
                Ok(line) => {
                    if tx.blocking_send(format!("{}\n", line)).is_err() {
                        break; // 通道关闭
                    }
                }
                Err(_) => break,
            }
        }
    });

    // 核心事件循环：同时处理“来自于 agy 的输出”和“来自于你键盘的输入”
    loop {
        tokio::select! {
            // 1. 监听 sshell 截获的 agy 运行输出
            Ok(event) = event_receiver.recv() => {
                match event {
                    ShellEvent::CommandStart(cmd) => {
                        // 内部 Hook 捕获到了 agy 的启动
                    }
                    ShellEvent::OutputChunk(chunk) => {
                        // 实时将 agy 的输出（包括颜色和进度条）同步给当前的屏幕
                        print!("{}", chunk);
                        io::stdout().flush().unwrap();
                    }
                    ShellEvent::CommandEnd(exit_code) => {
                        println!("\n\n🚀 [sshell 代理]: 底层的 agy 进程已经退出！退出码: {}", exit_code);
                        break;
                    }
                    ShellEvent::WaitingForInput => {}
                }
            }
            // 2. 监听你刚刚从键盘敲进去的内容
            Some(input) = rx.recv() => {
                // 安全地将你的输入灌入底层的 agy 的 PTY 通道中
                session.write_input(&input, false)?;
            }
        }
    }

    Ok(())
}
