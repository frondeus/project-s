use std::{
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    io::Write,
    panic::{catch_unwind, RefUnwindSafe},
    path::Path,
    // sync::mpsc::RecvTimeoutError,
};

use crate::{
    utils::{colordiff, diff, source_line, Section},
    Result,
};
use anyhow::{bail, Context};
use pulldown_cmark::CowStr;

struct TestCase<'a> {
    // Count in the file
    count: usize,
    source_line: usize,
    previous: HashMap<CowStr<'a>, &'a str>,
    args: Vec<String>,
    file: &'a str,
    entry: &'a Path,
    section: Option<Section<'a>>,
}

impl<'a> TestCase<'a> {
    fn case_line(&self) -> usize {
        let section = self.section.as_ref().expect("Section");
        // Ineffictient, counts all lines until the range again and again.
        first_line_from_offset(section.range.start, self.file)
    }

    fn has_arg(&self, arg: &str) -> bool {
        self.args.contains(&arg.to_string())
    }

    fn new(
        file: &'a str,
        entry: &'a Path,
        source_line: usize,
        first_name: CowStr<'a>,
        section: &'a str,
    ) -> Self {
        let mut previous = HashMap::new();
        previous.insert(first_name, section);
        Self {
            count: 0,
            source_line,
            previous,
            entry,
            file,
            args: vec![],
            section: None,
        }
    }
}

#[allow(dead_code)]
pub fn test_snapshots<R>(
    root: &str,
    source_names: &[&str],
    section_name: &str,
    test_fn: R,
) -> Result<()>
where
    R: RefUnwindSafe,
    R: Send + Fn(&str, &HashMap<CowStr, &str>, &HashSet<&str>) -> String,
    for<'a> &'a R: Send,
{
    let quiet = std::env::var("TEST_RUNNER_Q").unwrap_or_default() == "1";

    let path = crate::utils::project_root()?;

    let entries = glob::glob(&format!("{}/{root}/**/*.md", path.display()))?
        .collect::<Result<Vec<_>, _>>()?;
    let max_count = entries.len();

    let mut successes = 0;
    let mut failed = 0;

    // Remove patch files
    let patch_entries = glob::glob(&format!(
        "{}/{root}/**/*.{}*.patch",
        path.display(),
        section_name
    ))?
    .collect::<Result<Vec<_>, _>>()?;

    for entry in patch_entries {
        std::fs::remove_file(entry)?;
    }

    let files = entries
        .iter()
        .map(|e| std::fs::read_to_string(e).context("Could not open entry"))
        .collect::<Result<Vec<_>, _>>()?;

    let mut test_cases: Vec<TestCase> = vec![];

    for (entry_id, entry) in entries.iter().enumerate() {
        // let file = std::fs::read_to_string(&entry).context("Could not open entry")?;
        let md = crate::utils::load_markdown(&files[entry_id])?;

        let mut count = 0;
        for section in md {
            let name = section.name.to_string();
            let mut name_iter = name.split(' ');
            let name = name_iter.next().unwrap_or_default();
            match name {
                name if source_names.contains(&name) => {
                    let source_line = source_line(&files[entry_id], section.range.start);
                    test_cases.push(TestCase::new(
                        &files[entry_id],
                        entry,
                        source_line,
                        section.name,
                        section.section,
                    ));
                }
                name if name == section_name => {
                    let last = test_cases.last_mut().expect("test case");
                    if last.section.is_none() {
                        count += 1;
                        let args = name_iter.map(|s| s.to_string()).collect::<Vec<_>>();
                        last.section = Some(section);
                        last.args = args;
                        last.count = count;
                    }
                }
                _ => {
                    let Some(test_case) = test_cases.last_mut() else {
                        continue;
                    };
                    test_case.previous.insert(section.name, section.section);
                }
            }
        }
    }

    test_cases.retain(|t| t.section.is_some());
    if test_cases.iter().any(|t| t.has_arg("only")) {
        eprintln!("[[ Only running tests with only flag ]] ");
        eprintln!();
        eprintln!();
        test_cases.retain(|t| t.has_arg("only"));
    }
    test_cases.retain(|t| !t.has_arg("ignore"));

    for test_case in test_cases {
        let (code, source_name) = source_names
            .iter()
            .find_map(|source_name| {
                test_case
                    .previous
                    .get(*source_name)
                    .map(|code| (code, *source_name))
            })
            .expect("Source");

        // eprintln!("{source_name} - {code}");
        let test_fn = &test_fn;
        let previous = &test_case.previous;

        let args = test_case
            .args
            .iter()
            .map(|s| s.as_str())
            .collect::<HashSet<_>>();
        let actual = catch_unwind(|| test_fn(code, previous, &args));
        let actual = actual.unwrap_or_else(|_| "<Thread panicked>".to_string());

        if !quiet {
            print!("* {}:{}", test_case.entry.display(), test_case.case_line());
        }

        match assert_section(section_name, source_name, test_case, &actual, quiet) {
            Ok(_) => {
                if !quiet {
                    println!(" v");
                }
                successes += 1;
            }
            Err(e) => {
                println!("Error: {e}");
                failed += 1;
            }
        }
    }

    eprintln!("\nProcessed {section_name}: {max_count}, Succeded: {successes} Failed: {failed}",);
    if failed > 0 {
        bail!("Some tests failed");
    }

    Ok(())
}

fn count_backticks(slice: &str) -> usize {
    slice
        .chars()
        .fold((0, 0), |(max_count, current_count), c| {
            if c == '`' {
                let new_count = current_count + 1;
                (max_count.max(new_count), new_count)
            } else {
                (max_count, 0)
            }
        })
        .0
}

fn assert_section(
    name: &str,
    source_name: &str,
    test_case: TestCase,
    actual: &str,
    quiet: bool,
) -> Result<()> {
    let code = test_case.previous.get(source_name).expect("Source");

    let case_line = test_case.case_line();
    let expected = test_case.section.expect("expected");
    let count = test_case.count;
    let entry = test_case.entry;
    let file = test_case.file;
    let source_line = test_case.source_line;
    let range = expected.range;

    let fenced_with_code = |slice: &str, code: CowStr<'_>| -> String {
        let (fin, fout) = {
            let count = count_backticks(slice);
            let count = if count >= 3 { count + 1 } else { 3 };

            let backticks = "`".repeat(count);
            (format!("{backticks}{code}"), backticks)
        };
        format!("{fin}\n{slice}\n{fout}")
    };
    let fenced = |slice: &str| -> String { fenced_with_code(slice, expected.name) };

    let expected_name = if count > 1 {
        format!("{name}-{count:0>3}")
    } else {
        name.to_string()
    };

    // This is whitespace sensitive
    if expected.section != actual {
        let actual = fenced(actual);

        let new = format!("{}{}{}", &file[..range.start], &actual, &file[range.end..]);

        // For now
        let patch = diff(entry, expected_name.clone(), &new, true)?;
        if patch.is_empty() {
            // There is no real diff,
            return Ok(());
        }

        if !quiet {
            println!();

            colordiff(&patch)?;
        }

        let extension = format!("{expected_name}.patch");

        let new_file = entry.with_extension(extension);

        let mut new_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&new_file)
            .with_context(|| format!("Could create or open: {new_file:?}"))?;

        let expected_name = format!("{}:{}", entry.display(), case_line);
        new_file
            .write_all(format!("{expected_name}\n").as_bytes())
            .with_context(|| format!("Could not write to: {new_file:?}"))?;

        let source_name = format!("{source_name} {}:{source_line}", entry.display());
        let source_code = CowStr::Borrowed(source_name.as_str());

        new_file
            .write_all(fenced_with_code(code, source_code).as_bytes())
            .with_context(|| format!("Could not write to: {new_file:?}"))?;

        new_file
            .write_all(b"\n\n")
            .with_context(|| format!("Could not write to: {new_file:?}"))?;

        new_file
            .write_all(patch.as_bytes())
            .with_context(|| format!("Could not write to: {new_file:?}"))?;

        bail!("{expected_name} failed")
    } else {
        let rej_extension = format!("{expected_name}.rej");
        let rej_file = entry.with_extension(rej_extension);
        if rej_file.exists() {
            std::fs::remove_file(&rej_file)?;
        }
        Ok(())
    }
}

fn first_line_from_offset(byte_pos: usize, file: &str) -> usize {
    // byte_pos is guaranteed to be from inside of file.
    file[..byte_pos].chars().filter(|&c| c == '\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;

    #[test]
    fn test_runner() -> Result<()> {
        test_snapshots(
            "crates/test-runner/v2",
            &[""],
            "assert",
            |src: &str, _sections, _args| src.to_string(),
        )
    }

    use test_case::test_case;

    #[test_case("` ` `" => 1; "1")]
    #[test_case("` ` `" => 1; "2")]
    #[test_case("`` 3 ``" => 2; "3")]
    #[test_case("``` ddd ```" => 3; "4")]
    #[test_case("``` `" => 3; "5")]
    #[test_case("`` ``` ``" => 3; "6")]
    #[test_case("`` ````` ``" => 5; "7")]
    #[test_case("` `` ``` `` `" => 3; "8")]
    fn count_backticks_test(input: &str) -> usize {
        count_backticks(input)
    }
}
