//! A layer's GeoJSON read with `host.fetch` and added with `host.addLayer`, without reading it
//! into a `Value` in between: the SDK's `host::call` would parse the answer (its text and its
//! JSON, about twice the document) and write it out again, which for a megabyte or two takes
//! more than the fuel a command has. Here the answer stays bytes: its members are found by
//! skipping over their values, and the `json` one goes into the `host.addLayer` request as it is.

use rustgis_plugin::{Value, json, serde_json};

/// What became of a layer's address.
pub enum Fetched {
    /// Added: `host.addLayer`'s answer, `{"layer", "skipped"}`.
    Added(Value),
    /// Still coming (the web app): RustGIS runs the command again once it has.
    Pending,
}

/// Reads `url` with `host.fetch` and adds what it gives as layer `name`. `Ok(None)` when it can't
/// be had this way (more than the 2 MB `host.fetch` reads, a RustGIS without it, a network
/// error, an answer that isn't JSON), for `layer.addUrl` to add instead.
pub fn fetch_and_add(url: &str, name: &str) -> Result<Option<Fetched>, String> {
    let request = json!({"op": "host.fetch", "params": {"url": url}}).to_string();
    let Some(reply) = raw_call(request.as_bytes()) else { return Ok(None) };
    let Some(ok) = member(&reply, "ok") else { return Ok(None) };
    if member(ok, "pending") == Some(b"true".as_slice()) {
        return Ok(Some(Fetched::Pending));
    }
    let Some(doc) = member(ok, "json") else { return Ok(None) };
    // An ArcGIS server's error comes as JSON, with an HTTP 200.
    if let Some(err) = member(doc, "error") {
        let err: Value = serde_json::from_slice(err).unwrap_or_default();
        let text = err.get("message").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| err.to_string());
        return Err(format!("{url}: {text}"));
    }
    let mut add = Vec::with_capacity(doc.len() + name.len() + 64);
    add.extend_from_slice(br#"{"op":"host.addLayer","params":{"name":"#);
    add.extend_from_slice(Value::from(name).to_string().as_bytes());
    add.extend_from_slice(br#","geojson":"#);
    add.extend_from_slice(doc);
    add.extend_from_slice(b"}}");
    drop(reply);
    let reply: Value = serde_json::from_slice(&raw_call(&add).ok_or("RustGIS didn't answer")?).map_err(|e| e.to_string())?;
    match (reply.get("ok"), reply.get("error")) {
        (_, Some(e)) => Err(e.as_str().map(str::to_string).unwrap_or_else(|| e.to_string())),
        (Some(v), None) => Ok(Some(Fetched::Added(v.clone()))),
        _ => Err("RustGIS didn't answer".into()),
    }
}

/// A request to RustGIS, as the SDK's `host::call` sends it, and its reply's bytes.
#[cfg(target_arch = "wasm32")]
fn raw_call(request: &[u8]) -> Option<Vec<u8>> {
    #[link(wasm_import_module = "rustgis")]
    unsafe extern "C" {
        fn call(ptr: *const u8, len: usize) -> i32;
        fn result(ptr: *mut u8);
    }
    // SAFETY: the host reads `len` bytes at `ptr`, which `request` owns for the call.
    let n = unsafe { call(request.as_ptr(), request.len()) };
    let len = usize::try_from(n).ok()?;
    let mut buf: Vec<u8> = Vec::with_capacity(len);
    // SAFETY: the host writes exactly `len` bytes (the reply it measured) into `buf`'s capacity,
    // which is then marked initialized.
    unsafe {
        result(buf.as_mut_ptr());
        buf.set_len(len);
    }
    Some(buf)
}

/// Natively (the tests) there is no RustGIS to ask.
#[cfg(not(target_arch = "wasm32"))]
fn raw_call(_request: &[u8]) -> Option<Vec<u8>> {
    None
}

/// The value of member `key` of JSON object `obj`, as its bytes; `None` when `obj` isn't an
/// object or hasn't it.
pub fn member<'a>(obj: &'a [u8], key: &str) -> Option<&'a [u8]> {
    let mut i = ws(obj, 0);
    if obj.get(i) != Some(&b'{') {
        return None;
    }
    i = ws(obj, i + 1);
    while obj.get(i) == Some(&b'"') {
        let end = string_end(obj, i)?;
        let name = &obj[i..end];
        let start = ws(obj, ws(obj, end) + 1); // past the colon
        let value_end = value_end(obj, start)?;
        // Keys with escapes aren't ours: compared as written.
        if name.len() == key.len() + 2 && &name[1..name.len() - 1] == key.as_bytes() {
            return Some(&obj[start..value_end]);
        }
        i = ws(obj, value_end);
        if obj.get(i) != Some(&b',') {
            return None;
        }
        i = ws(obj, i + 1);
    }
    None
}

fn ws(b: &[u8], mut i: usize) -> usize {
    while b.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    i
}

/// Past the string that starts at `i` (its opening quote).
fn string_end(b: &[u8], mut i: usize) -> Option<usize> {
    i += 1;
    loop {
        match b.get(i)? {
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
}

/// Past the value that starts at `i`.
fn value_end(b: &[u8], i: usize) -> Option<usize> {
    match b.get(i)? {
        b'"' => string_end(b, i),
        b'{' | b'[' => {
            let (mut depth, mut i) = (0usize, i);
            loop {
                match b.get(i)? {
                    b'"' => {
                        i = string_end(b, i)?;
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(i + 1);
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
        _ => {
            let mut i = i;
            while b.get(i).is_some_and(|c| !matches!(c, b',' | b'}' | b']') && !c.is_ascii_whitespace()) {
                i += 1;
            }
            Some(i)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_found_by_skipping_values() {
        let reply = br#"{"ok": {"json": {"type": "FeatureCollection", "features": [{"properties": {"n": "a \"}]\" b"}}]}, "text": "{\"x\": 1}", "url": "https://x"}}"#;
        let ok = member(reply, "ok").unwrap();
        assert_eq!(member(ok, "url"), Some(br#""https://x""#.as_slice()));
        assert_eq!(member(ok, "text"), Some(br#""{\"x\": 1}""#.as_slice()));
        let doc = member(ok, "json").unwrap();
        let parsed: Value = serde_json::from_slice(doc).unwrap();
        assert_eq!(parsed["features"][0]["properties"]["n"], "a \"}]\" b");
        assert_eq!(member(doc, "error"), None);
        assert_eq!(member(br#"{"ok":{"pending":true,"url":"u"}}"#, "ok").and_then(|o| member(o, "pending")), Some(b"true".as_slice()));
        assert_eq!(member(br#"{"error":"no"}"#, "ok"), None);
        assert_eq!(member(b"[1]", "ok"), None);
        let err = br#"{"error": {"code": 400, "message": "Invalid query"}}"#;
        assert_eq!(member(err, "error"), Some(br#"{"code": 400, "message": "Invalid query"}"#.as_slice()));
    }
}
