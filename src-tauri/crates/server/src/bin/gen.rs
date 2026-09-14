//! 契约生成器（design D3 + IDR-03）：聚合 **contracts 剩余 stub + server 已实装 handler**
//! 两处 ApiDoc → 写 `contracts/openapi.json` 与 `contracts/tools/*.schema.json`
//! （$ref 生成期解引用，自包含 JSON Schema）。
//!
//! 确定性（spike S5）：经 `serde_json::Value`（BTreeMap）序列化 = 全层级字典序键排序；
//! 无时间戳、无随机内容；同源两次运行产物字节一致。
//! 注：本 bin 位于 server crate（聚合需要同时可见 contracts 与 server 的注解，
//! 依赖方向 server → contracts 单向，不构成环）。
//! 变换机械在 [`server::gendoc`]（可测模块），本 bin 只留装配。

use std::fs;

use contracts::tools;
use contracts::ApiDoc;
use utoipa::OpenApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = server::repo_root()?;

    // 0) 组装：contracts 剩余 stub + server 已实装 handler（IDR-03 聚合点）
    let mut doc = ApiDoc::openapi();
    doc.merge(server::ServerApiDoc::openapi());

    // 1) openapi.json（确定性键排序）
    let mut value = serde_json::to_value(&doc)?;
    server::gendoc::enforce_wire_discipline(&mut value);
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
        server::gendoc::deref_refs(&mut schema, &components, 0)?;
        server::gendoc::strip_null(&mut schema);
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
