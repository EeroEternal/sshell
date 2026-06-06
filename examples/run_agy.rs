use anyhow::Result;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use sshell::{ShellEvent, StructuredShell};
use std::io::{self, Read, Write};

struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let shell = StructuredShell::new();
    let mut event_receiver = shell.subscribe();

    println!("===================================================");
    println!("🚀 [sshell 代理]: 准备挂载本机实际安装的 `agy` 进程...");
    println!("说明：本次采用全透传 Raw Mode，完美支持 TUI 界面和快捷键交互！");
    println!("===================================================\n");

    shell.release_to_human();
    let mut session = shell.spawn_pty("agy")?;

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);

    // 开启宿主终端的 Raw Mode，拦截一切底层按键（不再有行缓冲和退格截获）
    enable_raw_mode()?;
    // 使用 Guard 确保即便程序崩溃也能恢复终端状态
    let _guard = RawModeGuard;

    // 单独线程：全盘监听标准输入的原始字节，并发给后台
    std::thread::spawn(move || {
        let mut stdin = io::stdin();
        let mut buf = [0u8; 1024];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let bytes = buf[..n].to_vec();
                    if tx.blocking_send(bytes).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    loop {
        tokio::select! {
            // 1. 监听 sshell 截获的 agy 运行输出
            Ok(event) = event_receiver.recv() => {
                match event {
                    ShellEvent::OutputChunk(chunk) => {
                        // 实时将 agy 的输出（包括颜色和光标移动的 ANSI）同步给当前屏幕
                        print!("{}", chunk);
                        io::stdout().flush().unwrap();
                    }
                    ShellEvent::CommandEnd(exit_code) => {
                        // 程序安全退出
                        drop(_guard); // 提前恢复终端
                        println!("\n\n🚀 [sshell 代理]: 底层的 agy 进程已经退出！退出码: {}", exit_code);
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
            // 2. 将宿主人类敲击的原生按键（方向键、快捷键）完全透传给 PTY
            Some(bytes) = rx.recv() => {
                let input_str = String::from_utf8_lossy(&bytes);
                session.write_input(&input_str, false)?;
            }
        }
    }
}
