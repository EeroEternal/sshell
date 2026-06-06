use anyhow::{anyhow, Result};
use serde_json::json;
use sshell::{ShellEvent, StructuredShell};
use std::env;
use std::io::{self, Write};

/// 真实调用 Gemini 大模型，根据终端的输出历史，自主决定要输入什么指令
async fn ask_llm(api_key: &str, terminal_output: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
        api_key
    );

    let prompt = format!(
        "你是一个极其聪明的 AI 程序员 Agent。现在，底层终端进程挂起了，正在等待输入。\n\
        请阅读以下由 sshell 捕获的终端旁路无损输出，并自主决定你应该输入什么动作（例如输入 '执行'、'y' 等）。\n\
        【重要】：你的回复将直接被灌入终端标准输入！请**只**回复你需要注入的内容本身，不要有任何解释，不要加引号！\n\n\
        Terminal Output:\n{}",
        terminal_output
    );

    let body = json!({
        "contents": [{
            "parts": [{"text": prompt}]
        }]
    });

    let res = client.post(&url).json(&body).send().await?;
    let json_res: serde_json::Value = res.json().await?;

    if let Some(text) = json_res["candidates"][0]["content"]["parts"][0]["text"].as_str() {
        Ok(text.trim().to_string())
    } else {
        Err(anyhow!("无法解析大模型响应: {:?}", json_res))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("===================================================");
    println!("🧠 [True Agent]: 正在唤醒真实 AI 大脑...");
    let api_key = env::var("GEMINI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        println!("⚠️  [True Agent]: 未检测到 GEMINI_API_KEY 环境变量！");
        println!("⚠️  [True Agent]: 为了让你看到流程，我将使用硬编码的模拟返回。");
        println!("💡 强烈建议你: export GEMINI_API_KEY=\"你的API_KEY\" 后重新运行本程序，体验真实的 LLM 终端接管！");
    } else {
        println!("✅ [True Agent]: 已成功连接到 Gemini 神经网络！");
    }
    println!("===================================================\n");

    let shell = StructuredShell::new();
    let mut event_receiver = shell.subscribe();

    shell.lock_for_agent();

    // 使用一个会挂起并等待交互的 bash 脚本
    let cmd = "echo 'Starting critical deployment...' && sleep 1 && echo -n 'Please enter action to perform (e.g., 执行): ' && read ans && echo '' && echo \"[Script] Executing your action: $ans\" && sleep 1";

    println!("🧠 [True Agent]: 派生高危交互命令: `{}`\n", cmd);
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

                // 发现终端挂起，触发大模型真实思考！
                if chunk.contains("Please enter action to perform") {
                    println!("\n\n🧠 [True Agent]: 发现终端界面挂起！正在收集完整的终端上下文发送给云端 LLM 进行真实推理...");

                    let decision = if api_key.is_empty() {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        "执行".to_string() // Fallback
                    } else {
                        // 【核心】：真实调用大模型！
                        ask_llm(&api_key, &captured_output).await.unwrap_or_else(|e| {
                            println!("❌ LLM 请求失败: {}", e);
                            "出错降级".to_string()
                        })
                    };

                    println!("🧠 [True Agent]: 🤖 大模型深度思考完毕！它的决策指令是: [{}]", decision);
                    println!("🧠 [True Agent]: 正在将大模型的决策自动灌入底层的 PTY 中...\n");

                    // 将大模型的决策真实灌入终端
                    session.write_input(&format!("{}\n", decision), true)?;
                }
            }
            ShellEvent::WaitingForInput => {}
            ShellEvent::CommandEnd(exit_code) => {
                println!("\n>> [Hook 事件抛出] 进程彻底结束，收到退出码: {}", exit_code);
                break;
            }
        }
    }

    println!("\n🧠 [True Agent]: 终端任务已完成！控制权已交还人类。");
    shell.release_to_human();

    Ok(())
}
