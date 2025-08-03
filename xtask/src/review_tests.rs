use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::Result;

pub fn run(root: &Path, llm: bool) -> Result {
    std::env::set_current_dir(root)?;

    let entries = glob::glob("docs/**/*.patch")?.collect::<Result<Vec<_>, _>>()?;

    let max_count = entries.len();
    println!();
    println!("Reviewing snapshots: {max_count} files found");
    println!();

    let mut already_rejected = 0;
    let mut new_cases: Vec<String> = Vec::with_capacity(entries.len());

    for (idx, actual) in entries.into_iter().enumerate() {
        let count = idx + 1;
        let actual_content = std::fs::read_to_string(&actual)?;

        println!("[{count}/{max_count}] Reviewing: {actual:?}");
        let rejected = actual.with_extension("rej");
        if rejected.exists() {
            let rejected_content = std::fs::read_to_string(&rejected)?;
            if actual_content == rejected_content {
                already_rejected += 1;
                std::fs::remove_file(actual)?;
                println!("\tRejected file is the same as the actual file");
                println!();
                continue;
            } else {
                std::fs::remove_file(&rejected)?;
            }
        }
        println!("-----");
        let mut colordiff = Command::new("colordiff")
            .arg("--nobanner")
            .stdin(Stdio::piped())
            .spawn()?;

        colordiff
            .stdin
            .as_mut()
            .ok_or("Could not attach colordiff STDIN")?
            .write_all(actual_content.as_bytes())?;

        colordiff.wait_with_output()?;

        println!("-----");
        let first_line = actual_content.lines().next().unwrap_or_default();
        println!("{first_line}");

        if llm {
            new_cases.push(format!("{} - from {}", actual.display(), first_line));
            continue;
        }
        loop {
            println!("[Aa]ccept, [Rr]eject or [Ss]kip");

            let mut choice = String::new();
            std::io::stdin().read_line(&mut choice)?;

            match choice.as_str().trim() {
                "A" | "a" => {
                    let dir = actual.parent().ok_or("Expected parent of snapshot file")?;

                    let mut patch = Command::new("patch")
                        .arg("--ignore-whitespace")
                        .current_dir(dir)
                        .stdin(Stdio::piped())
                        .spawn()?;
                    patch
                        .stdin
                        .as_mut()
                        .ok_or("Could not attach patch STDIN")?
                        .write_all(actual_content.as_bytes())?;

                    patch.wait_with_output()?;

                    std::fs::remove_file(actual)?;
                    if rejected.exists() {
                        std::fs::remove_file(&rejected)?;
                    }

                    break;
                }
                "R" | "r" => {
                    std::fs::copy(&actual, rejected)?;
                    std::fs::remove_file(actual)?;
                    break;
                }
                "S" | "s" => {
                    println!("Skipping {actual:?}");
                    break;
                }
                _ => continue,
            }
        }
    }

    // Remove original files
    let original_entries = glob::glob("docs/**/*.orig")?.collect::<Result<Vec<_>, _>>()?;

    for entry in original_entries {
        if let Err(e) = std::fs::remove_file(&entry) {
            eprintln!("Could not remove {entry:?}: {e}");
        }
    }

    if llm {
        println!();
        println!("--- SUMMARY ---");
        println!(
            "{already_rejected} entries already had rejected snapshot and content did not change"
        );
        println!("{} new entries are failing:", new_cases.len());
        for entry in new_cases {
            println!("* {entry}");
        }
    }

    Ok(())
}
