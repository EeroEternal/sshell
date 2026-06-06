use crate::ShellEvent;

/// 语义化 Shell 拦截器 (Semantic Hooks)
///
/// 依靠传统的轮询或进程锁去判断命令是否结束非常脆弱。现代 Shell (如 zsh/bash 结合 shell integration)
/// 可以在输出流中嵌入不可见的 OSC 133 (Operating System Command) 序列，
/// 明确汇报终端正在经历的生命周期。
pub struct SemanticHookParser;

impl SemanticHookParser {
    /// 实时扫描抓取到的 PTY 字节块，提取语义事件
    /// 这能让大模型获得如事件驱动一般精准的时机把控
    pub fn parse_chunk(chunk: &str) -> Vec<ShellEvent> {
        let mut events = Vec::new();

        // 匹配 OSC 133;B (Command Start 序列)
        // \x07 是 BEL (终端响铃) 字符，常作为 OSC 序列的结束符
        // \x1b\\ 是 ST (String Terminator)，也是常见的结束符
        if chunk.contains("\x1b]133;B\x07") || chunk.contains("\x1b]133;B\x1b\\") {
            events.push(ShellEvent::CommandStart("detected_command".to_string()));
        }

        // 匹配 OSC 133;D;<exit_code> (Command Finished 序列)
        // 这里简单处理 0 和非 0 的情况，实际应用中可用正则提取完整退出码
        if chunk.contains("\x1b]133;D;0\x07") || chunk.contains("\x1b]133;D;0\x1b\\") {
            events.push(ShellEvent::CommandEnd(0));
        } else if chunk.contains("\x1b]133;D;") {
            events.push(ShellEvent::CommandEnd(1)); // 泛指错误退出
        }

        // 匹配 OSC 133;A (Prompt Started - 等待用户输入)
        if chunk.contains("\x1b]133;A\x07") || chunk.contains("\x1b]133;A\x1b\\") {
            events.push(ShellEvent::WaitingForInput);
        }

        events
    }
}
