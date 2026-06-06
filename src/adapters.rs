/// 命令拦截与适配层 (API-First Adapters)
///
/// 这个模块的核心目标是尽可能避免直接将底层程序的终端报错文字发给 LLM。
/// 很多现代化的开发工具（如 cargo, npm）原生提供了结构化的 JSON 输出模式。
/// 我们在 Agent 将命令丢给 PTY 执行前进行拦截：如果有可用的结构化模式，
/// 自动追加相关参数，使得我们捕获到的底层输出（Shadow Stream）天然就是结构化的。

pub struct CommandAdapter;

impl CommandAdapter {
    /// 自动对已知命令进行“结构化拦截重写”
    pub fn adapt(raw_cmd: &str) -> String {
        let cmd_trimmed = raw_cmd.trim();

        // 拦截 Cargo 命令
        if cmd_trimmed.starts_with("cargo ") {
            if (cmd_trimmed.contains("build") || cmd_trimmed.contains("check") || cmd_trimmed.contains("test"))
                && !cmd_trimmed.contains("--message-format")
            {
                // 让 Rust 编译器以 JSON 格式吐出诊断信息
                return format!("{} --message-format=json", cmd_trimmed);
            }
        }

        // 拦截 NPM 命令
        if cmd_trimmed.starts_with("npm ") {
            if (cmd_trimmed.contains("install") || cmd_trimmed.contains("update"))
                && !cmd_trimmed.contains("--json")
            {
                return format!("{} --json", cmd_trimmed);
            }
        }

        // 默认不修改
        raw_cmd.to_string()
    }
}
