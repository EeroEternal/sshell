use anyhow::Result;
use portable_pty::{native_pty_system, Child, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub mod adapters;
pub mod hooks;
pub mod dom;

/// 终端状态，明确控制权，防止人机输入抢占
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TerminalOwnership {
    HumanDriving,
    AgentDriving,
}

/// 语义化 Shell 生命周期事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShellEvent {
    CommandStart(String),
    CommandEnd(i32),
    WaitingForInput,
    OutputChunk(String), // 用于机器旁路轨道 (Shadow Track) 捕获日志
}

/// 核心层：结构化 Shell (Structured Shell) 中间件
pub struct StructuredShell {
    ownership: Arc<Mutex<TerminalOwnership>>,
    event_tx: broadcast::Sender<ShellEvent>,
}

impl StructuredShell {
    /// 初始化 sshell 实例
    pub fn new() -> Self {
        // 创建一个用于分发 Shell 语义化事件的广播通道
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            ownership: Arc::new(Mutex::new(TerminalOwnership::HumanDriving)),
            event_tx,
        }
    }

    /// 返回一个事件订阅者，供外部（例如 UI 层或 Agent 监控服务）监听
    pub fn subscribe(&self) -> broadcast::Receiver<ShellEvent> {
        self.event_tx.subscribe()
    }

    /// 锁定终端，切断人类输入
    pub fn lock_for_agent(&self) {
        let mut own = self.ownership.lock().unwrap();
        *own = TerminalOwnership::AgentDriving;
    }

    /// 归还终端控制权给人类
    pub fn release_to_human(&self) {
        let mut own = self.ownership.lock().unwrap();
        *own = TerminalOwnership::HumanDriving;
    }

    /// 返回当前控制权状态
    pub fn ownership(&self) -> TerminalOwnership {
        self.ownership.lock().unwrap().clone()
    }

    /// 启动 PTY 并开启“双轨捕获 (Dual-Track)”后台任务
    pub fn spawn_pty(&self, raw_cmd: &str) -> Result<PtySession> {
        let pty_system = native_pty_system();
        
        // 1. 创建底层 PTY 对
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // 【关键修复】：调用 API 优先拦截器，自动追加 JSON 等结构化参数
        let adapted_cmd = adapters::CommandAdapter::adapt(raw_cmd);

        let mut command = CommandBuilder::new("bash");
        command.arg("-c");
        command.arg(&adapted_cmd);
        
        // 2. 挂载子进程
        let child = pair.slave.spawn_command(command)?;
        
        // 我们不再发出模拟的 Start 事件，而是交给底层的 Hook 去解析真实的启动信号
        
        // 3. 剥离 Master 读写句柄
        let master_reader = pair.master.try_clone_reader()?;
        let master_writer = pair.master.take_writer()?;
        let event_tx_clone = self.event_tx.clone();

        // 4. 后台任务：持久化抓取输出（机器轨 Shadow Stream）
        std::thread::spawn(move || {
            let mut reader = master_reader;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                        
                        // 发送原始抓取事件供上层选择性消费
                        let _ = event_tx_clone.send(ShellEvent::OutputChunk(chunk.clone()));

                        // 【关键修复】：调用 Semantic Hook 解析器，从底层输出中抽取真实生命周期事件并广播
                        let semantic_events = hooks::SemanticHookParser::parse_chunk(&chunk);
                        for event in semantic_events {
                            let _ = event_tx_clone.send(event);
                        }
                    }
                    Ok(_) => break, // EOF
                    Err(_) => break,
                }
            }
            // 兜底的退出事件
            let _ = event_tx_clone.send(ShellEvent::CommandEnd(0));
        });

        Ok(PtySession {
            child,
            writer: master_writer,
            ownership: self.ownership.clone(),
        })
    }
}

impl Default for StructuredShell {
    fn default() -> Self {
        Self::new()
    }
}

/// 一次 PTY 终端会话上下文
pub struct PtySession {
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    ownership: Arc<Mutex<TerminalOwnership>>,
}

impl PtySession {
    /// 向终端写入输入。内部实现【人机输入仲裁锁】机制
    /// 如果是 Agent 想要写入，或者 Human 在没有锁定的情况下写入，才能通行。
    pub fn write_input(&mut self, input: &str, is_agent: bool) -> Result<()> {
        let own = self.ownership.lock().unwrap().clone();
        
        if !is_agent && own == TerminalOwnership::AgentDriving {
            // 被 Agent 接管期间，拦截人类开发者的普通输入 (除非人类调用强行接管)
            anyhow::bail!("Input locked: Agent is currently driving the terminal.");
        }

        self.writer.write_all(input.as_bytes())?;
        self.writer.flush()?;
        Ok(())
    }
    
    /// 强制中断进程
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill()?;
        Ok(())
    }
}
