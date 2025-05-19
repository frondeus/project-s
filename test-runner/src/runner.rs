use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::Write,
    panic::{catch_unwind, RefUnwindSafe},
    path::Path,
    // sync::mpsc::RecvTimeoutError,
};

use crate::{
    utils::{colordiff, diff, Section},
    Result,
};
use anyhow::{bail, Context};
use pulldown_cmark::CowStr;

#[allow(dead_code)]
pub fn test_snapshots<R>(root: &str, section_name: &str, test_fn: R) -> Result<()>
where
    R: RefUnwindSafe,
    R: Send + Fn(&str, &HashMap<CowStr, &str>) -> String,
    for<'a> &'a R: Send,
{
    let path = crate::utils::project_root()?;

    let entries = glob::glob(&format!("{}/{root}/**/*.md", path.display()))?
        .collect::<Result<Vec<_>, _>>()?;
    let max_count = entries.len();

    let mut successes = 0;
    let mut skipped = 0;
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

    for entry in entries.into_iter() {
        let file = std::fs::read_to_string(&entry).context("Could not open entry")?;
        let md = crate::utils::load_markdown(&file)?;

        let mut previous = HashMap::<CowStr, &str>::new();
        let mut md = md.into_iter();
        let mut any_failed = false;
        let mut count = 0;
        loop {
            if let Some(section) = md.next() {
                let name = section.name.to_string();
                match name.as_str() {
                    "" => {
                        previous.clear();
                        previous.insert(section.name, section.section);
                    }
                    name if name == section_name => {
                        count += 1;
                        // eprintln!("Processing {entry:?} - {section_name} ({count})");
                        let expected = section;

                        let code = previous.get("").expect("Source");
                        // eprintln!("{code}");
                        let test_fn = &test_fn;
                        let previous = &previous;

                        // let (sender, receiver) = std::sync::mpsc::channel();
                        // std::thread::scope(move |s| {
                        //     let worker = s.spawn(move || {
                        let actual = catch_unwind(|| test_fn(code, previous));
                        let actual = actual.unwrap_or_else(|_| "<Thread panicked>".to_string());
                        //         sender.send(out).expect("Sender to be alive")
                        //     });
                        //     worker.join()
                        // })
                        // .map_err(|_| anyhow::anyhow!("Thread did not finish successfully"))?;

                        // let actual = thread.unwrap_or_else(|_| "<Thread panicked>".to_string());
                        // let actual = match receiver.recv_timeout(std::time::Duration::from_secs(5))
                        // {
                        //     Ok(o) => o,
                        //     Err(RecvTimeoutError::Timeout) => "<Thread timeout>".to_owned(),
                        //     Err(_) => "<Thread unexpected panic>".to_owned(),
                        // };

                        match assert_section(count, &entry, &file, &expected, &actual, code) {
                            Ok(_) => {
                                print!(".");
                            }
                            Err(e) => {
                                any_failed = true;
                                eprintln!("Error: {}", e);
                            }
                        }
                    }
                    _ => {
                        previous.insert(section.name, section.section);
                    }
                }
            } else if count == 0 {
                print!("s");
                skipped += 1;
                break;
            } else {
                if !any_failed {
                    successes += 1;
                } else {
                    failed += 1;
                }
                break;
            }
        }
    }

    eprintln!(
        "\nProcessed {section_name}: {max_count}, Succeded: {successes} Failed: {failed}, Skipped: {skipped}",
    );
    if failed > 0 {
        bail!("Some tests failed");
    }

    Ok(())
}

fn assert_section(
    count: usize,
    entry: &Path,
    file: &str,
    expected: &Section,
    actual: &str,
    code: &str,
) -> Result<()> {
    let fenced_without_code = |slice: &str| -> String {
        let fin = {
            let count = slice.chars().filter(|c| *c == '`').count();

            "`".repeat(count + 3)
        };
        format!("{fin}\n{}\n{fin}", slice)
    };
    let fenced = |slice: &str| -> String {
        let (fin, fout) = {
            let count = slice.chars().filter(|c| *c == '`').count();

            let backticks = "`".repeat(count + 3);
            (format!("{}{}", backticks, expected.name), backticks)
        };
        format!("{fin}\n{}\n{fout}", slice)
    };

    let expected_name = if count > 1 {
        format!("{}-{:0>3}", expected.name, count)
    } else {
        expected.name.to_string()
    };

    if expected.section != actual {
        let actual = fenced(actual);

        let new = format!(
            "{}{}{}",
            &file[..expected.range.start],
            &actual,
            &file[expected.range.end..]
        );

        let patch = diff(entry, expected_name.clone(), &new)?;

        colordiff(&patch)?;

        let extension = format!("{expected_name}.patch");

        let new_file = entry.with_extension(extension);

        let mut new_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&new_file)
            .with_context(|| format!("Could create or open: {new_file:?}"))?;

        new_file
            .write_all(fenced_without_code(code).as_bytes())
            .with_context(|| format!("Could not write to: {new_file:?}"))?;

        new_file
            .write_all(b"\n\n")
            .with_context(|| format!("Could not write to: {new_file:?}"))?;

        new_file
            .write_all(patch.as_bytes())
            .with_context(|| format!("Could not write to: {new_file:?}"))?;

        bail!("failed")
    } else {
        let rej_extension = format!("{expected_name}.rej");
        let rej_file = entry.with_extension(rej_extension);
        if rej_file.exists() {
            std::fs::remove_file(&rej_file)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Result;

    #[test]
    fn test_runner() -> Result<()> {
        test_snapshots("crates/test-runner/v2", "assert", |src, _sections| {
            src.to_string()
        })
    }
}
