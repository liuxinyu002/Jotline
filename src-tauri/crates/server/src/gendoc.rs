//! 契约生成机械（design D2）：从 gen bin 入库为可测模块。
//!
//! 职责单一：对 `serde_json::Value` 形态的 OpenAPI 文档做 Wire 纪律变换与
//! $ref 解引用。bin/gen.rs 只留 main() 装配（定位仓库根 → 组装 → 落盘），
//! 变换机械在此以 fixture 单测锚定（tests 见本文件尾部）。

/// $ref 解引用深度上限（循环引用防护）。
pub const MAX_DEREF_DEPTH: usize = 32;

/// Wire 纪律机械化（spike S2 结论落地）：utoipa 对 `Option<T>` 生成「可空联合」
/// （`oneOf: [{type: null}, X]` 或 `type: [T, null]`），与本库 Wire 纪律
/// 「可选 = 字段省略，禁止 required-nullable」冲突——生成期统一剔除 null 形态：
/// - `oneOf: [{type: "null"}, X]` → `X`
/// - `type: [T, "null"]` → `type: T`
/// - 查询参数剔除 null 后 `required: true` → `required: false`
pub fn enforce_wire_discipline(value: &mut serde_json::Value) {
    // 路径参数 / 查询参数：先剔 null，再修正 required
    if let Some(paths) = value.get_mut("paths").and_then(|p| p.as_object_mut()) {
        for (_path, item) in paths.iter_mut() {
            for (_method, op) in item.as_object_mut().into_iter().flatten() {
                if let Some(params) = op.get_mut("parameters").and_then(|p| p.as_array_mut()) {
                    for param in params.iter_mut() {
                        let mut removed = false;
                        if let Some(schema) = param.get_mut("schema") {
                            removed = strip_null(schema);
                        }
                        if removed {
                            if let Some(req) = param.get_mut("required") {
                                *req = serde_json::Value::Bool(false);
                            }
                        }
                    }
                }
            }
        }
    }
    // 全文档递归剔除（components / properties / items / oneOf …）
    strip_null(value);
}

/// 就地剔除 null 形态；返回是否发生剔除。
pub fn strip_null(value: &mut serde_json::Value) -> bool {
    let mut removed = false;
    match value {
        serde_json::Value::Object(map) => {
            // oneOf: [{type: "null"}, X] → X（X 的键提升到当前层级）
            if let Some(serde_json::Value::Array(variants)) = map.get_mut("oneOf") {
                if variants.len() == 2 && variants[0] == serde_json::json!({"type": "null"}) {
                    let x = variants.remove(1);
                    map.remove("oneOf");
                    removed = true;
                    if let serde_json::Value::Object(xm) = x {
                        for (k, v) in xm {
                            map.insert(k, v); // X 的键（含 $ref 兄弟 description）覆盖
                        }
                    }
                }
            }
            // type: [T, "null"] → 去除 "null"；单一元素坍缩为标量
            if let Some(serde_json::Value::Array(types)) = map.get_mut("type") {
                if types.iter().any(|t| t == "null") {
                    types.retain(|t| t != "null");
                    removed = true;
                    if types.len() == 1 {
                        let t = types.remove(0);
                        map.insert("type".to_string(), t);
                    }
                }
            }
            for (_k, v) in map.iter_mut() {
                removed |= strip_null(v);
            }
        }
        serde_json::Value::Array(items) => {
            for v in items.iter_mut() {
                removed |= strip_null(v);
            }
        }
        _ => {}
    }
    removed
}

/// 递归解引用 `#/components/schemas/<name>`（就地展开；$ref 兄弟键保留并覆盖）。
pub fn deref_refs(
    value: &mut serde_json::Value,
    components: &serde_json::Value,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_DEREF_DEPTH {
        return Err("工具 Schema $ref 解引用超深（疑似循环引用）".into());
    }
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(ref_path)) = map.get("$ref") {
                let name = ref_path
                    .strip_prefix("#/components/schemas/")
                    .ok_or(format!("不支持的 $ref 形态：{ref_path}"))?;
                let mut target = components
                    .get(name)
                    .ok_or(format!("$ref 目标不存在：{name}"))?
                    .clone();
                deref_refs(&mut target, components, depth + 1)?;
                // 兄弟键（如 description）覆盖展开结果
                let siblings: Vec<(String, serde_json::Value)> = map
                    .iter()
                    .filter(|(k, _)| k.as_str() != "$ref")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                if let serde_json::Value::Object(target_map) = &mut target {
                    for (k, v) in siblings {
                        target_map.insert(k, v);
                    }
                }
                *value = target;
                return Ok(());
            }
            for (_k, v) in map.iter_mut() {
                deref_refs(v, components, depth + 1)?;
            }
        }
        serde_json::Value::Array(items) => {
            for v in items.iter_mut() {
                deref_refs(v, components, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    /// oneOf 坍缩后 X 键提升覆盖兄弟键：X 的键（如 description）覆盖外层同名键。
    #[test]
    fn oneof_collapse_promotes_x_keys_over_siblings() {
        let mut v = json!({
            "description": "外层描述",
            "oneOf": [
                {"type": "null"},
                {"type": "string", "description": "X 描述", "maxLength": 10}
            ]
        });
        assert!(super::strip_null(&mut v));
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("oneOf"), "oneOf 应被坍缩移除");
        assert_eq!(obj.get("type"), Some(&json!("string")));
        assert_eq!(
            obj.get("description"),
            Some(&json!("X 描述")),
            "X 的键应覆盖外层兄弟键"
        );
        assert_eq!(obj.get("maxLength"), Some(&json!(10)));
    }

    /// `type: [T, null]` 坍缩为标量 `type: T`。
    #[test]
    fn nullable_type_array_collapses_to_scalar() {
        let mut v = json!({"type": ["integer", "null"], "minimum": 0});
        assert!(super::strip_null(&mut v));
        assert_eq!(v, json!({"type": "integer", "minimum": 0}));
    }

    /// $ref 解引用：目标展开，$ref 兄弟键（description）覆盖展开结果。
    #[test]
    fn deref_expands_ref_with_sibling_override() {
        let components = json!({
            "Note": {"type": "object", "description": "原描述", "properties": {"id": {"type": "string"}}}
        });
        let mut v = json!({
            "$ref": "#/components/schemas/Note",
            "description": "字段级描述"
        });
        super::deref_refs(&mut v, &components, 0).unwrap();
        let obj = v.as_object().unwrap();
        assert_eq!(
            obj.get("description"),
            Some(&json!("字段级描述")),
            "兄弟键应覆盖目标内同名键"
        );
        assert_eq!(obj.get("type"), Some(&json!("object")));
        assert!(obj.get("properties").is_some(), "目标内容应展开");
    }

    /// 循环 $ref 触发深度上限报错。
    #[test]
    fn cyclic_ref_hits_depth_limit() {
        let components = json!({
            "A": {"$ref": "#/components/schemas/B"},
            "B": {"$ref": "#/components/schemas/A"}
        });
        let mut v = json!({"$ref": "#/components/schemas/A"});
        let err = super::deref_refs(&mut v, &components, 0).unwrap_err();
        assert!(err.contains("超深"), "应报深度上限错误：{err}");
    }

    /// 查询参数剔 null 后 `required` 修正为 false；未剔 null 的参数不受影响。
    #[test]
    fn query_param_required_fixed_after_null_strip() {
        let mut v = json!({
            "paths": {
                "/api/notes": {
                    "get": {
                        "parameters": [
                            {
                                "name": "q",
                                "in": "query",
                                "required": true,
                                "schema": {"type": ["string", "null"]}
                            },
                            {
                                "name": "limit",
                                "in": "query",
                                "required": true,
                                "schema": {"type": "integer"}
                            }
                        ]
                    }
                }
            }
        });
        super::enforce_wire_discipline(&mut v);
        let params = &v["paths"]["/api/notes"]["get"]["parameters"];
        assert_eq!(
            params[0]["schema"]["type"],
            json!("string"),
            "剔 null 后坍缩为标量"
        );
        assert_eq!(
            params[0]["required"],
            json!(false),
            "剔 null 的参数 required 应修正为 false"
        );
        assert_eq!(
            params[1]["required"],
            json!(true),
            "未剔 null 的参数 required 不受影响"
        );
    }
}
