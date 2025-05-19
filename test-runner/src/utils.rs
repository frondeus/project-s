use std::{
    io::Write,
    ops::Range,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result};
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Parser, Tag};
use tempfile::tempdir;

pub fn project_root() -> Result<PathBuf> {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()?
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output)?.trim());
    Ok(cargo_path
        .parent()
        .ok_or(anyhow::anyhow!("Could not find parent to Cargo workspace"))?
        .to_path_buf())
}

#[derive(Debug)]
pub struct Section<'a> {
    pub name: CowStr<'a>,
    pub section: &'a str,
    pub range: Range<usize>,
}

pub fn load_markdown(file: &str) -> Result<Vec<Section<'_>>> {
    let parser = Parser::new(file);

    let mut entries = vec![];

    for (event, range) in parser.into_offset_iter() {
        if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(code))) = event {
            let block = &file[range.clone()];
            let stripped = strip_fences_offset(range.clone(), block);

            entries.push(Section {
                name: code,
                section: &file[stripped.clone()],
                range,
            });
        }
    }

    Ok(entries)
}

/// Calculate range of the string, by skipping first line and ignoring last line.
fn strip_fences_offset(range: Range<usize>, s: &str) -> Range<usize> {
    let mut lines = s.lines();
    let start = lines.next().unwrap_or_default().len() + 1 + range.start;
    let end = s.len() - lines.last().unwrap_or_default().len() - 1 + range.start;
    if end > start {
        start..end
    } else {
        start..start
    }
}

pub fn diff(a_path: &Path, b_name: String, b: &str) -> Result<String> {
    let parent = a_path.parent().ok_or(anyhow::anyhow!("No parent"))?;
    let tempdir = tempdir()?;

    let tempfile_path = tempdir.path().join(&b_name);
    let mut tempfile = std::fs::OpenOptions::new()
        .write(true)
        .read(true)
        .truncate(true)
        .create(true)
        .open(&tempfile_path)
        .with_context(|| format!("Could not open tempfile: {tempfile_path:?}"))?;

    tempfile
        .write_all(b.as_bytes())
        .with_context(|| format!("Could not write to tempfile: {tempfile_path:?}"))?;

    let diff = Command::new("diff")
        .current_dir(parent)
        .arg("-u")
        .arg(a_path)
        .arg(tempfile_path)
        .arg("--label")
        .arg(a_path)
        .arg("--label")
        .arg(b_name)
        .output()
        .context("Could not run diff command")?;

    let out = String::from_utf8(diff.stdout)?;

    Ok(out)
}

pub fn colordiff(patch: &str) -> Result<()> {
    let mut colordiff = Command::new("colordiff")
        .arg("--nobanner")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Could not run colordiff command")?;

    colordiff
        .stdin
        .as_mut()
        .ok_or(anyhow::anyhow!("Could not attach colordiff STDIN"))?
        .write_all(patch.as_bytes())?;

    let output = colordiff.wait_with_output()?;

    let out = String::from_utf8(output.stdout)?;
    eprintln!("{out}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// When parsing markdown fenced code block, we need to strip the fences
    fn strip_fences(s: &str) -> &str {
        let range = strip_fences_offset(0..0, s);

        &s[range]
    }

    #[test]
    fn strip_fences_tests() {
        let input = "```\nHello\nWorld\n```";
        let expected = "Hello\nWorld";

        assert_eq!(strip_fences(input), expected);

        // Different number of backticks
        let input = "````\nHello\nWorld\n````";
        let expected = "Hello\nWorld";

        assert_eq!(strip_fences(input), expected);

        // With language
        let input = "```lang\nHello\nWorld\n```";
        let expected = "Hello\nWorld";

        assert_eq!(strip_fences(input), expected);

        // With extra spaces
        let input = "``` lang \nHello\nWorld\n```";
        let expected = "Hello\nWorld";

        assert_eq!(strip_fences(input), expected);

        // With invalid number of fences
        let input = "````\nHello\nWorld\n```";
        let expected = "Hello\nWorld";

        assert_eq!(strip_fences(input), expected);

        // With empty string
        let input = "``` Foo\n```";
        let expected = "";

        assert_eq!(strip_fences(input), expected);
    }
}
