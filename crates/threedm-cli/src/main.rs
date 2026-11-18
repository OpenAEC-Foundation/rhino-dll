//! Command line front end of the 3dm reader.
//!
//! ```text
//! threedm info <file.3dm>                  the version, units, tables and objects by kind
//! threedm layers|materials|blocks|lights <file>
//!                                          a table
//! threedm objects <file> [limit] [offset]  objects with their attributes and user text
//! threedm object <file> <id>               one object with its geometry
//! threedm query <file> <json request>      any request of the C ABI (threedm.dll)
//! ```
//!
//! Answers are the JSON of the C ABI's `threedm_query`.

use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!("usage: threedm info|layers|materials|blocks|lights|plugins|plugin_data <file.3dm>\n       threedm objects <file.3dm> [limit] [offset]\n       threedm object <file.3dm> <id>\n       threedm query <file.3dm> <json request>");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(command), Some(path)) = (args.first(), args.get(1)) else { return usage() };
    let request = match command.as_str() {
        "info" | "layers" | "materials" | "blocks" | "lights" | "plugins" | "plugin_data" => serde_json::json!({ "query": command }),
        "objects" => {
            let number = |i: usize| args.get(i).and_then(|s| s.parse::<u64>().ok());
            let mut r = serde_json::json!({"query": "objects"});
            if let Some(limit) = number(2) {
                r["limit"] = limit.into();
            }
            if let Some(offset) = number(3) {
                r["offset"] = offset.into();
            }
            r
        }
        "object" => {
            let Some(id) = args.get(2) else { return usage() };
            serde_json::json!({"query": "object", "id": id})
        }
        "query" => {
            let Some(text) = args.get(2) else { return usage() };
            match serde_json::from_str(text) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("the request is not JSON: {e}");
                    return ExitCode::from(2);
                }
            }
        }
        _ => return usage(),
    };
    let doc = match threedm_read::Document::open(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let answer = threedm::query(&doc, &request.to_string());
    let ok = answer["ok"] == true;
    let shown = if ok { &answer["result"] } else { &answer };
    println!("{}", serde_json::to_string_pretty(shown).unwrap_or_default());
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
