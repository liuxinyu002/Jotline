//! 契约生成器（design D3 + IDR-03）：聚合 **contracts 剩余 stub + server 已实装 handler**
//! 两处 ApiDoc → 写 `contracts/openapi.json` 与 `contracts/tools/*.schema.json`
//! （$ref 生成期解引用，自包含 JSON Schema）。
//!
//! 确定性（spike S5）：经 `serde_json::Value`（BTreeMap）序列化 = 全层级字典序键排序；
//! 无时间戳、无随机内容；同源两次运行产物字节一致。
//! 注：本 bin 位于 server crate（聚合需要同时可见 contracts 与 server 的注解，
//! 依赖方向 server → contracts 单向，不构成环）。

use std::fs;
use std::path::{Path, PathBuf};

use contracts::tools;
use contracts::ApiDoc;
use utoipa::OpenApi;

/// $ref 解引用深度上限（循环引用防护）。
const MAX_DEREF_DEPTH: usize = 32;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = repo_root()?;

    // 0) 组装：contracts 剩余 stub + server 已实装 handler（IDR-03 聚合点）
    let mut doc = ApiDoc::openapi();
    doc.merge(server::ServerApiDoc::openapi());

    // 1) openapi.json（确定性键排序）
    let mut value = serde_json::to_value(&doc)?;
    enforce_wire_discipline(&mut value)?;
    let out_dir = repo_root.join("contracts");
    fs::create_dir_all(out_dir.join("tools"))?;
    let json = serde_json::to_string_pretty(&value)?;
    fs::write(out_dir.join("openapi.json"), format!("{json}\n"))?;

    // 2) 工具 Schema（$ref 解引用 → 自包含）
    let components = value
        .get("components")
        .and_then(|c| c.get("schemas"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    for tool in tools::tools() {
        let mut schema = serde_json::to_value((tool.params)())?;
        deref_refs(&mut schema, &components, 0)?;
        strip_null(&mut schema);
        let path = out_dir
            .join("tools")
            .join(format!("{}.schema.json", tool.name));
        fs::write(
            path,
            format!("{}\n", serde_json::to_string_pretty(&schema)?),
        )?;
    }

    println!("契约生成完成：contracts/openapi.json + contracts/tools/*.schema.json");
    Ok(())
}

/// 从 crate manifest（src-tauri/crates/server）上溯三级 = 仓库根。
fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.ancestors().nth(3).ok_or("无法定位仓库根目录")?;
    if !root.join("pnpm-workspace.yaml").exists() {
        return Err(format!("仓库根定位失败：{}", root.display()).into());
    }
    Ok(root.to_path_buf())
}

/// Wire 纪律机械化（spike S2 结论落地）：utoipa 对 `Option<T>` 生成「可空联合」
/// （`oneOf: [{type: null}, X]` 或 `type: [T, null]`），与本库 Wire 纪律
/// 「可选 = 字段省略，禁止 required-nullable」冲突——生成期统一剔除 null 形态：
/// - `oneOf: [{type: "null"}, X]` → `X`
/// - `type: [T, "null"]` → `type: T`
/// - 查询参数剔除 null 后 `required: true` → `required: false`
fn enforce_wire_discipline(
    value: &mut serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

/// 就地剔除 null 形态；返回是否发生剔除。
fn strip_null(value: &mut serde_json::Value) -> bool {
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
fn deref_refs(
    value: &mut serde_json::Value,
    components: &serde_json::Value,
    depth: usize,
) -> Result<(), Box<dyn std::error::Error>> {
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
