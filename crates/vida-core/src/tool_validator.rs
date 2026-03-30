use serde_json::Value;
use vida_providers::traits::{ToolCall, ToolDefinition};

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Tool '{0}' not found in available tools")]
    ToolNotFound(String),
    #[error("Invalid arguments for tool '{tool}': {message}")]
    InvalidArguments { tool: String, message: String },
}

pub fn validate_tool_call(
    call: &ToolCall,
    tools: &[ToolDefinition],
) -> Result<(), ValidationError> {
    let tool = tools
        .iter()
        .find(|tool| tool.name == call.name)
        .ok_or_else(|| ValidationError::ToolNotFound(call.name.clone()))?;

    validate_against_schema(&call.arguments, &tool.parameters).map_err(|message| {
        ValidationError::InvalidArguments {
            tool: call.name.clone(),
            message,
        }
    })
}

fn validate_against_schema(value: &Value, schema: &Value) -> Result<(), String> {
    let schema_type = schema.get("type").and_then(Value::as_str);

    if let Some(expected) = schema_type {
        match expected {
            "object" => validate_object(value, schema)?,
            "array" => validate_array(value, schema)?,
            "string" if !value.is_string() => return Err("expected string".to_string()),
            "number" if !value.is_number() => return Err("expected number".to_string()),
            "integer" if !value.as_i64().is_some() && !value.as_u64().is_some() => {
                return Err("expected integer".to_string())
            }
            "boolean" if !value.is_boolean() => return Err("expected boolean".to_string()),
            "null" if !value.is_null() => return Err("expected null".to_string()),
            _ => {}
        }
    }

    Ok(())
}

fn validate_object(value: &Value, schema: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "expected object".to_string())?;

    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for key in required.iter().filter_map(Value::as_str) {
            if !object.contains_key(key) {
                return Err(format!("missing required field '{key}'"));
            }
        }
    }

    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        for (key, prop_schema) in properties {
            if let Some(prop_value) = object.get(key) {
                validate_against_schema(prop_value, prop_schema)
                    .map_err(|e| format!("field '{key}' {e}"))?;
            }
        }
    }

    Ok(())
}

fn validate_array(value: &Value, schema: &Value) -> Result<(), String> {
    let array = value
        .as_array()
        .ok_or_else(|| "expected array".to_string())?;

    if let Some(item_schema) = schema.get("items") {
        for (idx, item) in array.iter().enumerate() {
            validate_against_schema(item, item_schema).map_err(|e| format!("item {idx} {e}"))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write_file_tool() -> ToolDefinition {
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Write a file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"]
            }),
        }
    }

    #[test]
    fn test_validate_valid_tool_call() {
        let call = ToolCall {
            id: "1".to_string(),
            name: "write_file".to_string(),
            arguments: json!({"path": "/tmp/test.txt", "content": "hello"}),
        };

        assert!(validate_tool_call(&call, &[write_file_tool()]).is_ok());
    }

    #[test]
    fn test_validate_missing_required_field() {
        let call = ToolCall {
            id: "1".to_string(),
            name: "write_file".to_string(),
            arguments: json!({"path": "/tmp/test.txt"}),
        };

        let err = validate_tool_call(&call, &[write_file_tool()]).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidArguments { .. }));
    }

    #[test]
    fn test_validate_unknown_tool() {
        let call = ToolCall {
            id: "1".to_string(),
            name: "unknown".to_string(),
            arguments: json!({}),
        };

        let err = validate_tool_call(&call, &[write_file_tool()]).unwrap_err();
        assert!(matches!(err, ValidationError::ToolNotFound(_)));
    }
}
