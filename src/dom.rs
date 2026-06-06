use serde::{Deserialize, Serialize};

/// 终端状态抽象语法树节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomNode {
    pub tag: String,
    pub content: String,
}

/// 终端 DOM 解析器 (Terminal DOM)
///
/// 相比于直接将乱码和无意义的控制字符发送给大模型，我们尝试解析 ANSI 转义序列，
/// 并将其转换为类似于 HTML/XML 的结构，保留颜色和语义。
pub struct TerminalDomParser;

impl TerminalDomParser {
    /// 一个轻量级的概念验证实现：
    /// 将红色的文字识别为 <error>，绿色的识别为 <success>，以此类推。
    /// 这样 LLM 收到的是 JSON 化的节点数组，而非一段带 `\x1b[31m` 的乱码文本。
    pub fn parse(raw_ansi_text: &str) -> Vec<DomNode> {
        let mut nodes = Vec::new();
        
        // 极简实现：判断段落主色调
        if raw_ansi_text.contains("\x1b[31m") {
            // 包含红色，可能是一段 Error
            nodes.push(DomNode {
                tag: "error".to_string(),
                // 清理掉终端控制字符
                content: raw_ansi_text.replace("\x1b[31m", "").replace("\x1b[0m", ""),
            });
        } else if raw_ansi_text.contains("\x1b[32m") {
            // 包含绿色，可能是一段 Success
            nodes.push(DomNode {
                tag: "success".to_string(),
                content: raw_ansi_text.replace("\x1b[32m", "").replace("\x1b[0m", ""),
            });
        } else {
            // 普通常规输出
            nodes.push(DomNode {
                tag: "text".to_string(),
                content: raw_ansi_text.to_string(),
            });
        }
        
        nodes
    }
}
