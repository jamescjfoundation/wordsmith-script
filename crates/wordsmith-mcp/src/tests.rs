// Modified by the Wordsmith Script contributors.
//! An MCP client doing realistic tasks through the protocol only, checking results via MCP.

use base64::Engine as _;
use serde_json::{Value, json};

use crate::{Headless, Server};

fn call(s: &mut Server, id: u64, method: &str, params: Value) -> Value {
    let line = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
    let r = s.handle_line(&line).expect("reply");
    serde_json::from_str(&r).expect("json")
}

fn tool(s: &mut Server, name: &str, args: Value) -> Value {
    let r = call(s, 9, "tools/call", json!({"name": name, "arguments": args}));
    let res = &r["result"];
    assert_eq!(res["isError"], false, "{name}: {res}");
    let text = res["content"][0]["text"].as_str().unwrap_or("null");
    serde_json::from_str(text).unwrap_or(Value::String(text.to_string()))
}

fn png_base64() -> String {
    base64::engine::general_purpose::STANDARD.encode(b"\x89PNG")
}

#[test]
fn lifecycle_and_tools() {
    let mut s = Server::new(Box::new(Headless::default()));
    let init = call(&mut s, 1, "initialize", json!({"protocolVersion": "2025-06-18"}));
    assert_eq!(init["result"]["serverInfo"]["name"], "wordsmith-script");
    let instructions = init["result"]["instructions"].as_str().unwrap_or_default().to_lowercase();
    assert!(!instructions.contains("artcraft") && !instructions.contains("discord") && !instructions.contains("wordcraft"), "{instructions}");
    assert!(s.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    let tools = call(&mut s, 2, "tools/list", json!({}));
    assert!(tools["result"]["tools"].as_array().unwrap().len() >= 5);
    let bad = call(&mut s, 3, "nope", json!({}));
    assert!(bad["error"].is_object());
    assert!(s.handle_line("{not json").unwrap().contains("parse error"));
}

#[test]
fn commands_are_listed_and_run_through_the_session() {
    let mut s = Server::new(Box::new(Headless::default()));
    let list = tool(&mut s, "list_commands", json!({"query": "undo"}));
    let ids: Vec<&str> = list.as_array().unwrap().iter().filter_map(|c| c["id"].as_str()).collect();
    assert!(ids.contains(&"edit.undo") && ids.contains(&"edit.undoHistory"), "{ids:?}");
    let history = tool(&mut s, "execute", json!({"command": "edit.undoHistory"}));
    assert_eq!(history["depth"], 0);
    let r = call(&mut s, 4, "tools/call", json!({"name": "execute", "arguments": {"command": "no.such"}}));
    assert_eq!(r["result"]["isError"], true);
}

#[test]
fn resources() {
    let mut s = Server::new(Box::new(Headless::default()));
    let l = call(&mut s, 1, "resources/list", json!({}));
    assert_eq!(l["result"]["resources"].as_array().unwrap().len(), 2);
    let r = call(&mut s, 2, "resources/read", json!({"uri": "wordsmith://document"}));
    assert!(r["result"]["contents"][0]["text"].as_str().unwrap().contains("paragraphs"));
    let r = call(&mut s, 3, "resources/read", json!({"uri": "wordsmith://commands"}));
    assert!(r["result"]["contents"][0]["text"].as_str().unwrap().contains("edit.undo"));
}

#[test]
fn headless_render_page_returns_a_png() {
    let mut s = Server::new(Box::new(Headless::default()));
    let r = call(&mut s, 5, "tools/call", json!({"name": "render_page", "arguments": {"page": 1, "scale": 0.25}}));
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert_eq!(r["result"]["content"][0]["type"], "image");
    let data = r["result"]["content"][0]["data"].as_str().unwrap();
    let png = base64::engine::general_purpose::STANDARD.decode(data).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
    let r = call(&mut s, 6, "tools/call", json!({"name": "render_page", "arguments": {"page": 99}}));
    assert_eq!(r["result"]["isError"], true);
}

#[test]
fn ui_tools_need_the_app_when_headless() {
    let mut s = Server::new(Box::new(Headless::default()));
    let r = call(&mut s, 7, "tools/call", json!({"name": "screenshot", "arguments": {}}));
    assert_eq!(r["result"]["isError"], true);
    assert!(r.to_string().contains("desktop app"), "{r}");
}

/// A backend standing in for a running app: it answers `ui.render` and `ui.screenshot` with a
/// PNG in the reply and records what it was asked.
struct FakeApp {
    calls: std::sync::Arc<std::sync::Mutex<Vec<(String, Value)>>>,
}

impl crate::backend::Backend for FakeApp {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.calls.lock().unwrap().push((method.to_string(), params.clone()));
        match method {
            "ui.render" | "ui.screenshot" => Ok(json!({"png": png_base64(), "width": 1, "height": 1})),
            other => Err(format!("unexpected {other}")),
        }
    }
    fn has_ui(&self) -> bool {
        true
    }
    fn describe(&self) -> String {
        "fake app".into()
    }
}

/// With an app attached, pictures travel inside the reply: the bridge never writes a file with a
/// predictable name into the shared temporary folder (another user could plant a link there).
#[test]
fn render_and_screenshot_through_the_app_use_no_temp_file() {
    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut s = Server::new(Box::new(FakeApp { calls: calls.clone() }));
    for (tool, method) in [("render_page", "ui.render"), ("screenshot", "ui.screenshot")] {
        // `screenshot` takes no arguments (the shared check refuses ones its schema does not list).
        let args = if tool == "render_page" { json!({"page": 1}) } else { json!({}) };
        let r = call(&mut s, 10, "tools/call", json!({"name": tool, "arguments": args}));
        let content = &r["result"]["content"];
        assert_eq!(r["result"]["isError"], false, "{tool}: {r}");
        assert_eq!(content[0]["type"], "image", "{tool}");
        assert_eq!(content[0]["data"], png_base64(), "{tool}");
        let (m, params) = calls.lock().unwrap().pop().expect("one call");
        assert_eq!(m, method);
        assert!(params.get("path").is_none(), "{tool} asked for a file: {params}");
    }
}
