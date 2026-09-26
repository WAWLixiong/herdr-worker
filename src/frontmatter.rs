// YAML 子集 frontmatter 解析器。
// 支持：`---` 分隔、`key: value` 标量（可带引号）、`[a, b, c]` 内联列表、`#` 注释行。
// 明确报错而非静默误读；产物文件对外保持合法 YAML。

use anyhow::{bail, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    List(Vec<Value>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct Frontmap {
    fields: Vec<(String, Value)>,
}

impl Frontmap {
    pub fn parse(fm: &str) -> Result<Self> {
        let mut fields = Vec::new();
        for (ln, raw) in fm.lines().enumerate() {
            let line = raw.trim_end();
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some(colon) = line.find(':') else {
                bail!("frontmatter 第 {} 行缺少 ':'：{}", ln + 1, line);
            };
            let key = line[..colon].trim().to_string();
            if key.is_empty() {
                bail!("frontmatter 第 {} 行 key 为空：{}", ln + 1, line);
            }
            fields.push((key, parse_value(line[colon + 1..].trim())?));
        }
        Ok(Frontmap { fields })
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty())
    }

    pub fn int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_int())
    }

    pub fn str_list(&self, key: &str) -> Vec<String> {
        match self.get(key) {
            Some(Value::List(items)) => items
                .iter()
                .map(|v| match v {
                    Value::Str(s) => s.clone(),
                    Value::Int(i) => i.to_string(),
                    Value::List(_) => String::new(),
                })
                .filter(|s| !s.is_empty())
                .collect(),
            Some(Value::Str(s)) if !s.is_empty() => vec![s.clone()],
            _ => Vec::new(),
        }
    }

    pub fn int_list(&self, key: &str) -> Vec<i64> {
        match self.get(key) {
            Some(Value::List(items)) => items.iter().filter_map(|v| v.as_int()).collect(),
            _ => Vec::new(),
        }
    }
}

fn parse_value(v: &str) -> Result<Value> {
    if v.is_empty() {
        return Ok(Value::Str(String::new()));
    }
    if v.starts_with('[') {
        if !v.ends_with(']') {
            bail!("列表缺少闭合 ']'：{v}");
        }
        let inner = &v[1..v.len() - 1];
        let mut items = Vec::new();
        for part in inner.split(',') {
            let p = part.trim();
            if p.is_empty() {
                continue;
            }
            items.push(parse_scalar(p)?);
        }
        return Ok(Value::List(items));
    }
    parse_scalar(v)
}

fn parse_scalar(s: &str) -> Result<Value> {
    let unquote = |q: char| -> Option<String> {
        if s.len() >= 2 && s.starts_with(q) && s.ends_with(q) {
            Some(s[1..s.len() - 1].to_string())
        } else {
            None
        }
    };
    if let Some(inner) = unquote('"').or_else(|| unquote('\'')) {
        return Ok(Value::Str(inner));
    }
    if let Ok(i) = s.parse::<i64>() {
        return Ok(Value::Int(i));
    }
    Ok(Value::Str(s.to_string()))
}

/// 拆出 frontmatter（不含 `---` 行）与正文。无 frontmatter 时返回 `(None, 原文)`。
pub fn split(src: &str) -> (Option<String>, String) {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let lines: Vec<&str> = src.split_inclusive('\n').collect();
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        return (None, src.to_string());
    }
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim_end() == "---" {
            return (Some(lines[1..i].concat()), lines[i + 1..].concat());
        }
    }
    (None, src.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_quotes_and_colons() {
        let fm = Frontmap::parse("title: 修复：登录页\nstatus: in-progress\n").unwrap();
        assert_eq!(fm.str("title"), Some("修复：登录页"));
        assert_eq!(fm.str("status"), Some("in-progress"));
    }

    #[test]
    fn quoted_values() {
        let fm = Frontmap::parse("title: \"a: b\"\nn: '42'\n").unwrap();
        assert_eq!(fm.str("title"), Some("a: b"));
        assert_eq!(fm.str("n"), Some("42"));
    }

    #[test]
    fn ints_and_lists() {
        let fm = Frontmap::parse("id: 0042\nblocked-by: [39, 41]\ntags: [bug, frontend]\n").unwrap();
        assert_eq!(fm.int("id"), Some(42));
        assert_eq!(fm.int_list("blocked-by"), vec![39, 41]);
        assert_eq!(fm.str_list("tags"), vec!["bug", "frontend"]);
    }

    #[test]
    fn comments_and_blanks_skipped() {
        let fm = Frontmap::parse("# 注释\n\npriority: high\n").unwrap();
        assert_eq!(fm.str("priority"), Some("high"));
        assert_eq!(fm.fields_len(), 1);
    }

    #[test]
    fn missing_colon_is_error() {
        assert!(Frontmap::parse("nope\n").is_err());
    }

    #[test]
    fn unclosed_list_is_error() {
        assert!(Frontmap::parse("tags: [a, b\n").is_err());
    }

    #[test]
    fn split_with_and_without_frontmatter() {
        let src = "---\nid: 1\n---\n正文第一行\n第二行\n";
        let (fm, body) = split(src);
        assert_eq!(fm.unwrap(), "id: 1\n");
        assert_eq!(body, "正文第一行\n第二行\n");

        let (fm, body) = split("就是正文");
        assert!(fm.is_none());
        assert_eq!(body, "就是正文");

        let (fm, _) = split("---\nid: 1\n没闭合");
        assert!(fm.is_none());
    }
}

impl Frontmap {
    pub fn fields_len(&self) -> usize {
        self.fields.len()
    }
}
