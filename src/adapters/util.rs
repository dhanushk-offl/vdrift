//! Shared file-format read/write helpers used by the ecosystem adapters.
//!
//! Every helper returns `Ok(None)` (or `Ok(false)`) when the version is
//! simply not present, and an `Adapter` error when the file is unreadable or
//! structurally broken — the caller decides how to treat "absent".

use crate::core::version::Version;
use crate::errors::{Result, VdriftError};
use quick_xml::Reader;
use quick_xml::Writer;
use quick_xml::events::Event;
use std::io::Cursor;
use std::path::Path;

pub fn read_text(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| VdriftError::Adapter(format!("cannot read {}: {e}", path.display())))
}

pub fn write_text(path: &Path, contents: &str) -> Result<()> {
    std::fs::write(path, contents)
        .map_err(|e| VdriftError::Adapter(format!("cannot write {}: {e}", path.display())))
}

// ----------------------------------------------------------------- JSON

/// Reads the top-level `version` string of a JSON object.
pub fn read_json_version(path: &Path) -> Result<Option<Version>> {
    let text = read_text(path)?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| VdriftError::Adapter(format!("invalid JSON in {}: {e}", path.display())))?;
    Ok(value
        .get("version")
        .and_then(|v| v.as_str())
        .and_then(|s| Version::parse(s).ok()))
}

/// Sets the top-level `version` of a JSON object, preserving the rest.
pub fn write_json_version(path: &Path, version: &Version) -> Result<()> {
    let text = read_text(path)?;
    let mut value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| VdriftError::Adapter(format!("invalid JSON in {}: {e}", path.display())))?;
    if !value.is_object() {
        return Err(VdriftError::Adapter(format!(
            "{} is not a JSON object",
            path.display()
        )));
    }
    value["version"] = serde_json::Value::String(version.to_string());
    let rendered = serde_json::to_string_pretty(&value).map_err(|e| {
        VdriftError::Adapter(format!("failed to serialize {}: {e}", path.display()))
    })?;
    write_text(path, &(rendered + "\n"))
}

// ----------------------------------------------------------------- YAML

/// Reads a (possibly nested) string value from a YAML document.
pub fn read_yaml_keys(path: &Path, keys: &[&str]) -> Result<Option<Version>> {
    let text = read_text(path)?;
    let value: serde_yaml::Value = serde_yaml::from_str(&text)
        .map_err(|e| VdriftError::Adapter(format!("invalid YAML in {}: {e}", path.display())))?;
    let mut cur = &value;
    for key in keys {
        match cur.get(*key) {
            Some(v) => cur = v,
            None => return Ok(None),
        }
    }
    Ok(cur.as_str().and_then(|s| Version::parse(s).ok()))
}

/// Sets a (possibly nested) string value in a YAML document.
pub fn write_yaml_keys(path: &Path, keys: &[&str], version: &Version) -> Result<()> {
    let text = read_text(path)?;
    let mut value: serde_yaml::Value = serde_yaml::from_str(&text)
        .map_err(|e| VdriftError::Adapter(format!("invalid YAML in {}: {e}", path.display())))?;
    let mut cur = &mut value;
    for (i, key) in keys.iter().enumerate() {
        if i == keys.len() - 1 {
            cur[*key] = serde_yaml::Value::String(version.to_string());
        } else if !cur[*key].is_mapping() {
            cur[*key] = serde_yaml::Value::Mapping(Default::default());
            cur = &mut cur[*key];
        } else {
            cur = &mut cur[*key];
        }
    }
    let rendered = serde_yaml::to_string(&value).map_err(|e| {
        VdriftError::Adapter(format!("failed to serialize {}: {e}", path.display()))
    })?;
    write_text(path, &rendered)
}

// ----------------------------------------------------------------- TOML

/// Reads a (possibly nested) string value from a TOML document.
pub fn read_toml_keys(path: &Path, keys: &[&str]) -> Result<Option<Version>> {
    let text = read_text(path)?;
    let value: toml::Value = toml::from_str(&text)
        .map_err(|e| VdriftError::Adapter(format!("invalid TOML in {}: {e}", path.display())))?;
    let mut cur = &value;
    for key in keys {
        match cur.get(*key) {
            Some(v) => cur = v,
            None => return Ok(None),
        }
    }
    Ok(cur.as_str().and_then(|s| Version::parse(s).ok()))
}

// ------------------------------------------------------- line-keyed text

/// Cleans a raw value fragment: trims, drops trailing comments/punctuation
/// and surrounding quotes.
fn clean_value(value: &str) -> &str {
    let mut v = value.trim();
    if let Some(pos) = v.find(" #") {
        v = v[..pos].trim();
    }
    if let Some(pos) = v.find("\t#") {
        v = v[..pos].trim();
    }
    v = v.trim_end_matches([',', ';']);
    v = v.trim();
    if v.len() >= 2 {
        let first = v.as_bytes()[0];
        let last = v.as_bytes()[v.len() - 1];
        if matches!((first, last), (b'"', b'"') | (b'\'', b'\'')) {
            v = &v[1..v.len() - 1];
        }
    }
    v.trim()
}

/// Extracts the version from a line that starts with any of the given key
/// prefixes, e.g. `version = "1.2.3"`, `__version__ = "1.2.3"`,
/// `spec.version = '1.2.3'`, `var Version = "1.2.3"`.
fn current_on_line(trimmed: &str, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(rest) = trimmed.strip_prefix(key)
            && let Ok(v) = Version::parse(clean_value(rest))
        {
            return Some(v.to_string());
        }
    }
    None
}

/// Reads the first line (anywhere in the file) that starts with one of the
/// key prefixes and carries a parseable version.
pub fn read_line_version(path: &Path, keys: &[&str]) -> Result<Option<Version>> {
    let text = read_text(path)?;
    for line in text.lines() {
        if let Some(v) = current_on_line(line.trim(), keys) {
            return Ok(Some(Version::parse(&v)?));
        }
    }
    Ok(None)
}

/// Replaces the version on the first matching line, preserving indentation
/// and quoting. `old` pins the exact version to replace when known.
pub fn write_line_version(
    path: &Path,
    keys: &[&str],
    old: Option<&Version>,
    version: &Version,
) -> Result<bool> {
    let text = read_text(path)?;
    let new_str = version.to_string();
    let old_str = old.map(|o| o.to_string());
    let mut lines: Vec<String> = text.lines().map(String::from).collect();

    for line in lines.iter_mut() {
        let trimmed = line.trim();
        if !keys.iter().any(|k| trimmed.starts_with(k)) {
            continue;
        }
        if let Some(old) = &old_str {
            if line.contains(old) {
                *line = line.replace(old, &new_str);
                write_text(path, &(lines.join("\n") + "\n"))?;
                return Ok(true);
            }
        } else if let Some(cur) = current_on_line(trimmed, keys) {
            *line = line.replace(&cur, &new_str);
            write_text(path, &(lines.join("\n") + "\n"))?;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Bounds (inclusive start, exclusive end) of a `[section]` block in a line list.
fn section_bounds(lines: &[String], section: &str) -> Option<(usize, usize)> {
    let start = lines
        .iter()
        .position(|l| l.trim() == format!("[{section}]"))?;
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.trim_start().starts_with('['))
        .map(|i| start + 1 + i)
        .unwrap_or(lines.len());
    Some((start, end))
}

/// Reads the version from the first matching line inside a `[section]`.
pub fn read_section_keys(path: &Path, section: &str, keys: &[&str]) -> Result<Option<Version>> {
    let text = read_text(path)?;
    let lines: Vec<String> = text.lines().map(String::from).collect();
    let Some((_, end)) = section_bounds(&lines, section) else {
        return Ok(None);
    };
    let header = section_bounds(&lines, section).map(|(s, _)| s).unwrap_or(0);
    for line in &lines[header + 1..end] {
        if let Some(v) = current_on_line(line.trim(), keys) {
            return Ok(Some(Version::parse(&v)?));
        }
    }
    Ok(None)
}

/// Replaces the version on the first matching line inside a `[section]`.
pub fn write_section_keys(
    path: &Path,
    section: &str,
    keys: &[&str],
    old: Option<&Version>,
    version: &Version,
) -> Result<bool> {
    let text = read_text(path)?;
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let Some((start, end)) = section_bounds(&lines, section) else {
        return Ok(false);
    };
    let new_str = version.to_string();
    let old_str = old.map(|o| o.to_string());

    for i in start + 1..end {
        let trimmed = lines[i].trim();
        if !keys.iter().any(|k| trimmed.starts_with(k)) {
            continue;
        }
        if let Some(old) = &old_str {
            if lines[i].contains(old) {
                lines[i] = lines[i].replace(old, &new_str);
                write_text(path, &(lines.join("\n") + "\n"))?;
                return Ok(true);
            }
        } else if let Some(cur) = current_on_line(trimmed, keys) {
            lines[i] = lines[i].replace(&cur, &new_str);
            write_text(path, &(lines.join("\n") + "\n"))?;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Replaces a `version = "..."` line inside `[start, end)` of `lines`.
/// Used by the Cargo lockfile update to stay inside a single `[[package]]`.
pub fn replace_section_version(
    lines: &mut [String],
    start: usize,
    end: usize,
    old: Option<&str>,
    new: &str,
) -> bool {
    for i in start..end.min(lines.len()) {
        let trimmed = lines[i].trim();
        if !(trimmed.starts_with("version =") || trimmed.starts_with("version=")) {
            continue;
        }
        if let Some(old) = old {
            let expected = format!("version = \"{old}\"");
            if trimmed != expected && trimmed != format!("version=\"{old}\"") {
                continue;
            }
        }
        let indent = lines[i]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect::<String>();
        lines[i] = format!("{indent}version = \"{new}\"");
        return true;
    }
    false
}

// ------------------------------------------------------------------ XML

/// Reads the `<version>` text that is a direct child of `<project>` in a Maven
/// POM. Property placeholders (`${...}`) yield `Ok(None)`.
pub fn read_pom_version(path: &Path) -> Result<Option<Version>> {
    let text = read_text(path)?;
    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut buf = Vec::new();
    let mut found: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => stack.push(e.local_name().as_ref().to_vec()),
            Ok(Event::Empty(_)) => {}
            Ok(Event::End(_)) => {
                stack.pop();
            }
            Ok(Event::Text(t))
                if stack.len() == 2 && stack[0] == b"project" && stack[1] == b"version" =>
            {
                let s = t.decode().unwrap_or_default().trim().to_string();
                if !s.is_empty() {
                    found = Some(s);
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    match found {
        Some(s) => Ok(Version::parse(&s).ok()),
        None => Ok(None),
    }
}

/// Replaces the direct `<version>` child of `<project>` in a Maven POM while
/// preserving the rest of the file byte-for-byte.
pub fn write_pom_version(path: &Path, version: &Version) -> Result<()> {
    let text = read_text(path)?;
    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut buf = Vec::new();
    let mut replaced = false;

    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| VdriftError::Adapter(format!("invalid XML in {}: {e}", path.display())))?;
        match event {
            Event::Start(e) => {
                let name = e.local_name().as_ref().to_vec();
                stack.push(name);
                writer.write_event(Event::Start(e)).map_err(|e| {
                    VdriftError::Adapter(format!("failed to write {}: {e}", path.display()))
                })?;
            }
            Event::End(e) => {
                stack.pop();
                writer.write_event(Event::End(e)).map_err(|e| {
                    VdriftError::Adapter(format!("failed to write {}: {e}", path.display()))
                })?;
            }
            Event::Text(t) => {
                if !replaced
                    && stack.len() == 2
                    && stack[0] == b"project"
                    && stack[1] == b"version"
                    && !t.decode().unwrap_or_default().trim().is_empty()
                {
                    writer
                        .write_event(Event::Text(quick_xml::events::BytesText::new(
                            version.to_string().as_str(),
                        )))
                        .map_err(|e| {
                            VdriftError::Adapter(format!("failed to write {}: {e}", path.display()))
                        })?;
                    replaced = true;
                } else {
                    writer.write_event(Event::Text(t)).map_err(|e| {
                        VdriftError::Adapter(format!("failed to write {}: {e}", path.display()))
                    })?;
                }
            }
            Event::Eof => break,
            other => writer.write_event(other).map_err(|e| {
                VdriftError::Adapter(format!("failed to write {}: {e}", path.display()))
            })?,
        }
        buf.clear();
    }

    if !replaced {
        return Err(VdriftError::Adapter(format!(
            "no direct <version> under <project> in {}",
            path.display()
        )));
    }
    let bytes = writer.into_inner().into_inner();
    std::fs::write(path, bytes)
        .map_err(|e| VdriftError::Adapter(format!("cannot write {}: {e}", path.display())))
}
