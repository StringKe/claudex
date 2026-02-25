use std::collections::HashMap;

use serde_json::{json, Value};

/// OpenAI 工具名最大长度
pub const MAX_TOOL_NAME_LEN: usize = 64;

/// 工具名映射（截断名 → 原始名）
pub type ToolNameMap = HashMap<String, String>;

/// 截断过长的工具名，保持可辨识性
pub fn truncate_tool_name(name: &str) -> String {
    if name.len() <= MAX_TOOL_NAME_LEN {
        return name.to_string();
    }
    // 取前 55 字符 + "_" + 8 字符 hash
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    let hash = format!("{:08x}", hasher.finish());
    format!("{}_{}", &name[..MAX_TOOL_NAME_LEN - 9], &hash[..8])
}

/// SSE 格式化
pub fn format_sse(event: &str, data: &Value) -> String {
    format!(
        "event: {event}\ndata: {}\n\n",
        serde_json::to_string(data).unwrap_or_default()
    )
}

/// API key 预览（显示首尾各 4 字符）
pub fn format_key_preview(key: &str) -> String {
    if key.is_empty() {
        "(empty)".to_string()
    } else if key.len() > 8 {
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    } else {
        "***".to_string()
    }
}

/// 构造 Anthropic 格式的错误 JSON
pub fn to_anthropic_error(status: u16, message: &str) -> Value {
    let error_type = match status {
        401 => "authentication_error",
        403 => "permission_error",
        404 => "not_found_error",
        429 => "rate_limit_error",
        _ => "invalid_request_error",
    };
    json!({
        "type": "error",
        "error": {
            "type": error_type,
            "message": message,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_truncate_short_name_unchanged() {
        assert_eq!(truncate_tool_name("get_weather"), "get_weather");
    }

    #[test]
    fn test_truncate_exactly_64_unchanged() {
        let name = "a".repeat(64);
        assert_eq!(truncate_tool_name(&name), name);
    }

    #[test]
    fn test_truncate_65_chars() {
        let name = "a".repeat(65);
        let result = truncate_tool_name(&name);
        assert_eq!(result.len(), 64);
        assert!(result.starts_with("aaaa"));
        assert!(result.contains('_'));
    }

    #[test]
    fn test_truncate_preserves_determinism() {
        let name = "mcp__very_long_server_name__extremely_long_tool_function_name_here_v2";
        let r1 = truncate_tool_name(name);
        let r2 = truncate_tool_name(name);
        assert_eq!(r1, r2);
        assert_eq!(r1.len(), 64);
    }

    #[test]
    fn test_format_sse() {
        let data = json!({"type": "test"});
        let result = format_sse("my_event", &data);
        assert!(result.starts_with("event: my_event\ndata: "));
        assert!(result.ends_with("\n\n"));
        assert!(result.contains("\"type\":\"test\""));
    }

    #[test]
    fn test_format_key_preview_empty() {
        assert_eq!(format_key_preview(""), "(empty)");
    }

    #[test]
    fn test_format_key_preview_short() {
        assert_eq!(format_key_preview("12345678"), "***");
    }

    #[test]
    fn test_format_key_preview_long() {
        assert_eq!(format_key_preview("sk-abcd1234efgh5678"), "sk-a...5678");
    }

    #[test]
    fn test_to_anthropic_error() {
        let err = to_anthropic_error(401, "invalid key");
        assert_eq!(err["error"]["type"], "authentication_error");
        assert_eq!(err["error"]["message"], "invalid key");
    }
}
