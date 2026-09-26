// 写路径：对 frontmatter 的单字段定点改写（保留文件其余内容字节不动），保证 round-trip 稳定。

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// 把 frontmatter 中 `key:` 行替换为 `key: value`；frontmatter 缺该字段则插到闭合 `---` 前。
/// 文件无 frontmatter 时报错（任务文件必有）。
pub fn set_field(path: &Path, key: &str, value: &str) -> Result<()> {
    let src = fs::read_to_string(path).with_context(|| format!("读取 {}", path.display()))?;
    let mut lines: Vec<String> = src.split_inclusive('\n').map(String::from).collect();
    // 定位 frontmatter 区
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        bail!("{} 缺少 frontmatter", path.display());
    }
    let close = lines
        .iter()
        .skip(1)
        .position(|l| l.trim_end() == "---")
        .map(|i| i + 1)
        .ok_or_else(|| anyhow::anyhow!("{} frontmatter 未闭合", path.display()))?;
    let new_line = format!("{key}: {value}\n");
    let target = lines[1..close]
        .iter()
        .position(|l| l.trim_start().starts_with(&format!("{key}:")))
        .map(|i| i + 1);
    match target {
        Some(i) => lines[i] = new_line,
        None => lines.insert(close, new_line),
    }
    fs::write(path, lines.concat()).with_context(|| format!("写回 {}", path.display()))
}

/// 删除 frontmatter 中 `key:` 行（不存在则不动）；正文里的同前缀文本不受影响
pub fn remove_field(path: &Path, key: &str) -> Result<()> {
    let src = fs::read_to_string(path).with_context(|| format!("读取 {}", path.display()))?;
    let mut lines: Vec<String> = src.split_inclusive('\n').map(String::from).collect();
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        bail!("{} 缺少 frontmatter", path.display());
    }
    let Some(close) = lines
        .iter()
        .skip(1)
        .position(|l| l.trim_end() == "---")
        .map(|i| i + 1)
    else {
        bail!("{} frontmatter 未闭合", path.display());
    };
    let prefix = format!("{key}:");
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    for (i, line) in lines.drain(..).enumerate() {
        let in_fm = i > 0 && i < close;
        if in_fm && line.trim_start().starts_with(&prefix) {
            continue;
        }
        out.push(line);
    }
    fs::write(path, out.concat()).with_context(|| format!("写回 {}", path.display()))
}

pub fn utc_now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_field_replaces_and_inserts() {
        let dir = std::env::temp_dir().join(format!("herdr-worker-write-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("task.md");
        fs::write(
            &p,
            "---\nid: 0001\ntitle: 板能开\nstatus: todo\n---\n正文第一行\n第二行\n",
        )
        .unwrap();

        set_field(&p, "status", "review").unwrap();
        let out = fs::read_to_string(&p).unwrap();
        assert!(out.contains("status: review\n"));
        assert!(out.contains("id: 0001\n"), "id 行保持零填充原样");
        assert!(out.contains("title: 板能开\n"), "无关行不动");
        assert!(out.ends_with("正文第一行\n第二行\n"), "正文不动");

        set_field(&p, "order", "3").unwrap();
        let out = fs::read_to_string(&p).unwrap();
        assert!(out.contains("order: 3\n"));
        assert!(out.matches("---\n").count() == 2);

        set_field(&p, "status", "done").unwrap();
        let out = fs::read_to_string(&p).unwrap();
        assert!(out.contains("status: done\n"));
        assert!(!out.contains("status: review"));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn remove_field_deletes_only_in_frontmatter() {
        let dir = std::env::temp_dir().join(format!("herdr-worker-rm-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("task.md");
        fs::write(
            &p,
            "---\nid: 0001\nstatus: todo\norder: 2\n---\n正文提到 order: 无妨\n",
        )
        .unwrap();
        remove_field(&p, "order").unwrap();
        let out = fs::read_to_string(&p).unwrap();
        assert!(!out.contains("order: 2"));
        assert!(out.contains("正文提到 order: 无妨"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn timestamp_shape() {
        let t = utc_now_rfc3339();
        assert_eq!(t.len(), 20);
        assert!(t.ends_with('Z'));
        assert_eq!(t.as_bytes()[4], b'-');
        assert_eq!(t.as_bytes()[10], b'T');
    }
}
