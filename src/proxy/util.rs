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

/// 将 profile 的 extra_body 深度合并进翻译后的请求体：
/// 两边都是对象时递归合并，否则以 extra 的值覆盖。
/// 顶层 extra 不是对象时不做任何修改（配置错误由 validate 提示）。
pub fn merge_extra_body(body: &mut Value, extra: &Value) {
    let (Some(base), Some(ext)) = (body.as_object_mut(), extra.as_object()) else {
        return;
    };
    for (k, v) in ext {
        let merged_nested = match base.get_mut(k) {
            Some(slot) if slot.is_object() && v.is_object() => {
                merge_extra_body(slot, v);
                true
            }
            _ => false,
        };
        if !merged_nested {
            base.insert(k.clone(), v.clone());
        }
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

    #[test]
    fn test_merge_extra_body_inserts_missing_key() {
        let mut body = json!({"model": "gpt-5.3-codex", "input": []});
        merge_extra_body(&mut body, &json!({"reasoning": {"effort": "high"}}));
        assert_eq!(body["reasoning"]["effort"], "high");
        assert_eq!(body["model"], "gpt-5.3-codex");
    }

    #[test]
    fn test_merge_extra_body_deep_merges_objects() {
        let mut body = json!({"reasoning": {"summary": "auto"}});
        merge_extra_body(&mut body, &json!({"reasoning": {"effort": "high"}}));
        // 已有的兄弟字段保留，新字段合入
        assert_eq!(body["reasoning"]["summary"], "auto");
        assert_eq!(body["reasoning"]["effort"], "high");
    }

    #[test]
    fn test_merge_extra_body_overwrites_scalars() {
        let mut body = json!({"temperature": 1.0, "enable_thinking": false});
        merge_extra_body(
            &mut body,
            &json!({"enable_thinking": true, "thinking_budget": 32768}),
        );
        assert_eq!(body["enable_thinking"], true);
        assert_eq!(body["thinking_budget"], 32768);
        assert_eq!(body["temperature"], 1.0);
    }

    #[test]
    fn test_merge_extra_body_object_replaces_scalar() {
        let mut body = json!({"reasoning": "none"});
        merge_extra_body(&mut body, &json!({"reasoning": {"effort": "low"}}));
        assert_eq!(body["reasoning"]["effort"], "low");
    }

    #[test]
    fn test_merge_extra_body_non_object_extra_is_noop() {
        let mut body = json!({"model": "m"});
        merge_extra_body(&mut body, &json!("not-an-object"));
        assert_eq!(body, json!({"model": "m"}));
    }
}
