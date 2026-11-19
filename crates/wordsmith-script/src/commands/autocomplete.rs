// SPDX-License-Identifier: MIT OR Apache-2.0
//! Autocomplete commands: suggest, accept, edit the lists, switch them, order them, rebuild.

use serde_json::{Value, json};
use wordsmith_engine::{CmdError, CmdResult, CommandSpec, Selection, Session, cmd::sel_result, p};

use crate::autocomplete::{Autocomplete, ListKind, Order, rebuild, suggest};
use crate::element::Elements;

fn list_param(v: &Value) -> Result<ListKind, CmdError> {
    let name = p::req_str(v, "list")?;
    ListKind::parse(name).ok_or_else(|| CmdError::Params(format!("no list `{name}` (characters, extensions, intros, locations, times, transitions)")))
}

/// `autocomplete.suggest`: what would be offered at the caret.
fn suggest_cmd(s: &mut Session, _: &Value) -> CmdResult {
    if !s.sel.is_collapsed() {
        return Ok(json!({"candidates": []}));
    }
    Ok(match suggest(&s.doc, &s.sel.focus) {
        Some((ctx, candidates)) => json!({"list": ctx.list, "start": ctx.start, "prefix": ctx.prefix, "candidates": candidates}),
        None => json!({"candidates": []}),
    })
}

/// `autocomplete.accept {candidate}`: replace the typed prefix with an offered entry. Only an
/// entry the caret's list offers right now is accepted.
fn accept(s: &mut Session, v: &Value) -> CmdResult {
    let want = p::req_str(v, "candidate")?;
    let at = s.sel.focus.clone();
    let (ctx, candidates) = suggest(&s.doc, &at).ok_or_else(|| CmdError::Disabled("nothing is offered here".into()))?;
    let pick = candidates.iter().find(|c| c.as_str() == want).ok_or_else(|| CmdError::Params(format!("`{want}` is not offered here")))?.clone();
    let casing = Elements::of_doc(&s.doc).of_para(s.doc.para_at(&at).ok_or_else(|| CmdError::Failed("no paragraph".into()))?).casing;
    let mut text = casing.apply(&pick);
    let para = s.doc.para_mut(at.story, &at.path)?;
    // An extension closes its bracket.
    if ctx.list == ListKind::Extensions && !para.text.get(at.off..).unwrap_or("").trim_start().starts_with(')') {
        text.push(')');
    }
    let props = para.props_at(ctx.start).clone();
    para.delete(ctx.start, at.off)?;
    para.insert_text(ctx.start, &text, &props)?;
    para.touch();
    s.sel = Selection::caret(wordsmith_doc::Pos { off: ctx.start + text.len(), ..at });
    sel_result(s)
}

fn lists(s: &mut Session, _: &Value) -> CmdResult {
    let ac = Autocomplete::of_doc(&s.doc);
    let lists: Vec<Value> = ListKind::ALL
        .into_iter()
        .map(|k| {
            let l = ac.list(k);
            json!({"list": k, "name": k.name(), "enabled": l.enabled, "order": l.order, "entries": ac.entries(&s.doc, k)})
        })
        .collect();
    Ok(json!({"lists": lists, "tabInsertsExtension": ac.tab_inserts_extension, "guessNextSpeaker": ac.guess_next_speaker}))
}

fn add(s: &mut Session, v: &Value) -> CmdResult {
    let kind = list_param(v)?;
    let mut ac = Autocomplete::of_doc(&s.doc);
    let added = ac.add(&mut s.doc, kind, p::req_str(v, "term")?);
    ac.store(&mut s.doc);
    Ok(json!({"added": added}))
}

fn remove(s: &mut Session, v: &Value) -> CmdResult {
    let kind = list_param(v)?;
    let mut ac = Autocomplete::of_doc(&s.doc);
    let removed = ac.remove(&mut s.doc, kind, p::req_str(v, "term")?);
    ac.store(&mut s.doc);
    Ok(json!({"removed": removed}))
}

fn rename(s: &mut Session, v: &Value) -> CmdResult {
    let kind = list_param(v)?;
    let (from, to) = (p::req_str(v, "from")?, p::req_str(v, "to")?);
    let mut ac = Autocomplete::of_doc(&s.doc);
    if !ac.entries(&s.doc, kind).iter().any(|e| e.eq_ignore_ascii_case(from)) {
        return Err(CmdError::Params(format!("`{from}` is not in the list")));
    }
    if kind != ListKind::Intros
        && let Some(list) = ac.lists.get_mut(&kind)
        && let Some(i) = list.entries.iter().position(|e| e.eq_ignore_ascii_case(from))
    {
        // Keep its place in a custom order.
        list.entries.remove(i);
        let to = to.split_whitespace().collect::<Vec<_>>().join(" ");
        if to.is_empty() || to.len() > crate::autocomplete::MAX_ENTRY_LEN {
            return Err(CmdError::Params("`to` must be 1 to 100 bytes".into()));
        }
        list.entries.insert(i, to);
    } else {
        ac.remove(&mut s.doc, kind, from);
        ac.add(&mut s.doc, kind, to);
    }
    ac.store(&mut s.doc);
    Ok(json!({"renamed": true}))
}

/// `autocomplete.move {list, term, to}`: put an entry at position `to` (custom order).
fn move_entry(s: &mut Session, v: &Value) -> CmdResult {
    let kind = list_param(v)?;
    let term = p::req_str(v, "term")?;
    let to = p::u64(v, "to").ok_or_else(|| CmdError::Params("`to` (a position) is required".into()))? as usize;
    let mut ac = Autocomplete::of_doc(&s.doc);
    let list = ac.lists.entry(kind).or_default();
    let i = list.entries.iter().position(|e| e.eq_ignore_ascii_case(term)).ok_or_else(|| CmdError::Params(format!("`{term}` is not in the list")))?;
    let e = list.entries.remove(i);
    list.entries.insert(to.min(list.entries.len()), e);
    list.order = Order::Custom;
    ac.store(&mut s.doc);
    Ok(json!({"order": "custom"}))
}

/// `autocomplete.configure {list?, enabled?, order?, tabInsertsExtension?, guessNextSpeaker?}`.
fn configure(s: &mut Session, v: &Value) -> CmdResult {
    let mut ac = Autocomplete::of_doc(&s.doc);
    if v.get("list").is_some() {
        let kind = list_param(v)?;
        let list = ac.lists.entry(kind).or_default();
        if let Some(e) = v.get("enabled") {
            list.enabled = e.as_bool().ok_or_else(|| CmdError::Params("`enabled` must be true or false".into()))?;
        }
        if let Some(o) = v.get("order") {
            list.order = serde_json::from_value(o.clone()).map_err(|_| CmdError::Params("`order` is alphabetical, custom or speeches".into()))?;
        }
    }
    for (key, slot) in [("tabInsertsExtension", &mut ac.tab_inserts_extension), ("guessNextSpeaker", &mut ac.guess_next_speaker)] {
        if let Some(x) = v.get(key) {
            *slot = x.as_bool().ok_or_else(|| CmdError::Params(format!("`{key}` must be true or false")))?;
        }
    }
    ac.store(&mut s.doc);
    lists(s, &Value::Null)
}

pub fn specs() -> Vec<CommandSpec> {
    const LOC: &str = "Tools › Autocomplete";
    vec![
        CommandSpec::new("autocomplete.suggest", "Suggestions", LOC, suggest_cmd).pure(),
        CommandSpec::new("autocomplete.accept", "Accept Suggestion", LOC, accept).params(r#"{"candidate": string}"#),
        CommandSpec::new("autocomplete.lists", "Autocomplete Lists", LOC, lists).pure(),
        CommandSpec::new("autocomplete.add", "Add to List", LOC, add)
            .params(r#"{"list": characters|extensions|intros|locations|times|transitions, "term": string}"#),
        CommandSpec::new("autocomplete.remove", "Remove from List", LOC, remove).params(r#"{"list": list, "term": string}"#),
        CommandSpec::new("autocomplete.rename", "Rename in List", LOC, rename).params(r#"{"list": list, "from": string, "to": string}"#),
        CommandSpec::new("autocomplete.move", "Move in List", LOC, move_entry).params(r#"{"list": list, "term": string, "to": position}"#),
        CommandSpec::new("autocomplete.configure", "Autocomplete Options", LOC, configure).params(
            r#"{"list"?: list, "enabled"?: bool, "order"?: alphabetical|custom|speeches, "tabInsertsExtension"?: bool, "guessNextSpeaker"?: bool}"#,
        ),
        CommandSpec::new("autocomplete.rebuild", "Rebuild Lists", LOC, |s, _| {
            rebuild(&mut s.doc);
            lists(s, &Value::Null)
        }),
    ]
}
