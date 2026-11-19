// Modified by the Wordsmith Script contributors.
//! MCP tools: names, schemas and implementations on top of a [`Backend`].

use serde_json::{Value, json};

use crate::backend::Backend;

/// An image tool result from a reply carrying base64 PNG in `png`.
fn png_result(v: &Value, info: &Value) -> ToolResult {
    match v.get("png").and_then(Value::as_str) {
        Some(png) if !png.is_empty() => ToolResult::image(png.to_string(), info),
        _ => ToolResult::error("the reply carried no picture"),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolResult {
    pub content: Vec<Value>,
    pub is_error: bool,
}

impl ToolResult {
    pub fn text(t: impl Into<String>) -> Self {
        Self { content: vec![json!({"type": "text", "text": t.into()})], is_error: false }
    }
    pub fn json(v: &Value) -> Self {
        Self::text(serde_json::to_string_pretty(v).unwrap_or_default())
    }
    pub fn error(t: impl Into<String>) -> Self {
        Self { content: vec![json!({"type": "text", "text": t.into()})], is_error: true }
    }
    pub fn image(png_base64: String, info: &Value) -> Self {
        Self {
            content: vec![json!({"type": "image", "data": png_base64, "mimeType": "image/png"}), json!({"type": "text", "text": info.to_string()})],
            is_error: false,
        }
    }
    pub fn to_value(&self) -> Value {
        json!({"content": self.content, "isError": self.is_error})
    }
}

fn s(desc: &str) -> Value {
    json!({"type": "string", "description": desc})
}
fn n(desc: &str) -> Value {
    json!({"type": "number", "description": desc})
}
fn obj(props: Value, req: &[&str]) -> Value {
    json!({"type": "object", "properties": props, "required": req})
}
fn tool(name: &str, title: &str, desc: &str, schema: Value, ro: bool) -> Value {
    json!({"name": name, "title": title, "description": desc, "inputSchema": schema, "annotations": {"title": title, "readOnlyHint": ro, "openWorldHint": false}})
}

/// Every MCP tool with its title, description, input schema and annotations (`tools/list`).
pub fn tool_definitions() -> Value {
    json!([
        tool(
            "list_commands",
            "List commands",
            "Every Wordsmith Script command: id, label, menu location, shortcut and params. Filter with `query`.",
            obj(json!({"query": s("Case-insensitive filter on id/label/location")}), &[]),
            true
        ),
        tool(
            "execute",
            "Run a command",
            "Run one command by id with params, e.g. {command:\"edit.undo\"}. list_commands lists the ids and their params.",
            obj(json!({"command": s("Command id"), "params": {"type": "object", "description": "Command parameters"}}), &["command"]),
            false
        ),
        tool(
            "batch",
            "Run several commands",
            "Run commands in order; stops at the first error unless keepGoing.",
            obj(
                json!({"commands": {"type": "array", "items": {"type": "object", "properties": {"command": {"type": "string"}, "params": {"type": "object"}}, "required": ["command"]}}, "keepGoing": {"type": "boolean"}}),
                &["commands"]
            ),
            false
        ),
        tool("inspect_document", "Inspect script", "The script's text, structure and selection.", obj(json!({}), &[]), true),
        tool(
            "render_page",
            "Render page",
            "Render a page to PNG, as it prints, to look at the result.",
            obj(json!({"page": n("1-based page"), "scale": n("Pixels per point (default 1)")}), &[]),
            true
        ),
        tool("screenshot", "Screenshot app", "Screenshot of the whole app window (desktop app only).", obj(json!({}), &[]), true),
        tool(
            "click",
            "Click",
            "Click at window coordinates (desktop app only).",
            obj(
                json!({"x": n("x"), "y": n("y"), "count": n("1-3 clicks"), "button": s("left (default) or right"), "shift": {"type": "boolean"}, "alt": {"type": "boolean"}, "cmd": {"type": "boolean"}, "ctrl": {"type": "boolean"}, "command": {"type": "boolean"}}),
                &["x", "y"]
            ),
            false
        ),
        tool(
            "key",
            "Press key",
            "Press a key with modifiers (desktop app only), e.g. {key:\"B\", cmd:true}.",
            obj(
                json!({"key": s("Key name"), "shift": {"type": "boolean"}, "alt": {"type": "boolean"}, "cmd": {"type": "boolean"}, "ctrl": {"type": "boolean"}, "command": {"type": "boolean"}}),
                &["key"]
            ),
            false
        ),
        tool("ui_inspect", "Inspect UI", "UI state: panes, dialog, page rectangles on screen, caret (desktop app only).", obj(json!({}), &[]), true),
    ])
}

fn exec(b: &mut dyn Backend, id: &str, params: Value) -> Result<Value, String> {
    b.call("engine.execute", json!({"command": id, "params": params}))
}

fn wrap(r: Result<Value, String>) -> ToolResult {
    match r {
        Ok(v) => ToolResult::json(&v),
        Err(e) => ToolResult::error(e),
    }
}

/// Run tool `name` with arguments `a` through backend `b` (`tools/call`); unknown tools and
/// failed calls are error results, never panics.
pub fn call_tool(b: &mut dyn Backend, name: &str, a: &Value) -> ToolResult {
    let st = |k: &str| a.get(k).and_then(Value::as_str);
    match name {
        "list_commands" => {
            let q = st("query").unwrap_or("").to_lowercase();
            match b.call("engine.commands", json!({})) {
                Ok(Value::Array(v)) => {
                    let f: Vec<Value> = v
                        .into_iter()
                        .filter(|c| {
                            q.is_empty()
                                || ["id", "label", "location"]
                                    .iter()
                                    .any(|k| c.get(*k).and_then(Value::as_str).is_some_and(|x| x.to_lowercase().contains(&q)))
                        })
                        .collect();
                    ToolResult::json(&Value::Array(f))
                }
                Ok(v) => ToolResult::json(&v),
                Err(e) => ToolResult::error(e),
            }
        }
        "execute" => match st("command") {
            Some(c) => wrap(exec(b, c, a.get("params").cloned().unwrap_or(json!({})))),
            None => ToolResult::error("missing `command`"),
        },
        "batch" => {
            let keep = a.get("keepGoing").and_then(Value::as_bool).unwrap_or(false);
            let mut out = Vec::new();
            for c in a.get("commands").and_then(Value::as_array).cloned().unwrap_or_default() {
                let id = c.get("command").and_then(Value::as_str).unwrap_or("");
                let r = exec(b, id, c.get("params").cloned().unwrap_or(json!({})));
                let failed = r.is_err();
                out.push(match r {
                    Ok(v) => json!({"command": id, "ok": true, "result": v}),
                    Err(e) => json!({"command": id, "ok": false, "error": e}),
                });
                if failed && !keep {
                    return ToolResult { content: vec![json!({"type": "text", "text": Value::Array(out).to_string()})], is_error: true };
                }
            }
            ToolResult::json(&Value::Array(out))
        }
        "inspect_document" => wrap(b.call("document.inspect", json!({}))),
        // Pictures travel inside the reply (`png`), headless or through the app: no file with a
        // predictable name in the shared temporary folder.
        "render_page" => {
            let params = json!({"page": a.get("page").cloned().unwrap_or(json!(1)), "scale": a.get("scale").cloned().unwrap_or(json!(1.0))});
            match b.call("ui.render", params) {
                Ok(v) => png_result(&v, &json!({"width": v["width"], "height": v["height"], "pages": v["pages"]})),
                Err(e) => ToolResult::error(e),
            }
        }
        "screenshot" => match b.call("ui.screenshot", json!({})) {
            Ok(v) => png_result(&v, &json!({"width": v["width"], "height": v["height"]})),
            Err(e) => ToolResult::error(e),
        },
        "click" => wrap(b.call("ui.click", a.clone())),
        "key" => wrap(b.call("ui.key", a.clone())),
        "ui_inspect" => wrap(b.call("ui.inspect", json!({}))),
        other => ToolResult::error(format!("unknown tool `{other}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A backend that keeps what it was asked for, so a test can read the forwarded params.
    struct Recording {
        seen: Vec<(String, Value)>,
        fail: Option<&'static str>,
    }

    impl Backend for Recording {
        fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
            let id = params.get("command").and_then(Value::as_str).unwrap_or("").to_string();
            self.seen.push((method.to_string(), params));
            if self.fail == Some(id.as_str()) { Err(format!("{id} failed")) } else { Ok(json!({"ok": true})) }
        }
        fn has_ui(&self) -> bool {
            false
        }
        fn describe(&self) -> String {
            "recording".into()
        }
    }

    #[test]
    fn execute_forwards_the_command_and_its_params() {
        let mut b = Recording { seen: Vec::new(), fail: None };
        let r = call_tool(&mut b, "execute", &json!({"command": "edit.undo", "params": {"x": 1}}));
        assert!(!r.is_error);
        assert_eq!(b.seen, vec![("engine.execute".to_string(), json!({"command": "edit.undo", "params": {"x": 1}}))]);
        assert!(call_tool(&mut b, "execute", &json!({})).is_error, "a missing command is an error");
    }

    #[test]
    fn batch_stops_at_the_first_error_unless_told_to_keep_going() {
        let cmds = json!([{"command": "a"}, {"command": "bad"}, {"command": "c"}]);
        let mut b = Recording { seen: Vec::new(), fail: Some("bad") };
        let r = call_tool(&mut b, "batch", &json!({"commands": cmds}));
        assert!(r.is_error);
        assert_eq!(b.seen.len(), 2);
        let mut b = Recording { seen: Vec::new(), fail: Some("bad") };
        let r = call_tool(&mut b, "batch", &json!({"commands": cmds, "keepGoing": true}));
        assert!(!r.is_error);
        assert_eq!(b.seen.len(), 3);
    }

    #[test]
    fn unknown_tools_are_error_results() {
        let mut b = Recording { seen: Vec::new(), fail: None };
        assert!(call_tool(&mut b, "open_document", &json!({})).is_error);
        assert!(b.seen.is_empty());
    }

    #[test]
    fn every_tool_has_a_schema_and_none_is_word_specific() {
        let tools = tool_definitions();
        let tools = tools.as_array().expect("the tools are a list");
        for t in tools {
            assert_eq!(t["inputSchema"]["type"], "object", "{t}");
            let text = t.to_string().to_lowercase();
            assert!(!text.contains("wordcraft") && !text.contains("ribbon"), "{t}");
        }
    }
}
