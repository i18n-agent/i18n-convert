mod cli;

use clap::Parser;
use cli::Cli;
use i18n_convert::convert::check_data_loss;
use i18n_convert::formats::FormatRegistry;
use i18n_convert::ir::WarningSeverity;
use std::fs;
use std::io::{self, Write};

fn main() {
    let cli = Cli::parse();
    let registry = FormatRegistry::new();

    if cli.list_formats {
        println!("Supported formats:");
        for entry in registry.list() {
            let exts = entry.extensions.join(", ");
            println!("  {:<20} {} ({})", entry.id, entry.name, exts);
        }
        return;
    }

    let input = cli.input.as_ref().expect("input is required");
    let to = cli.to.as_ref().expect("--to is required");

    // Read input
    let content = fs::read(input).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {e}", input);
        std::process::exit(1);
    });

    // Detect input format
    let ext = std::path::Path::new(input)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let detected = registry.detect(&ext, &content);
    if detected.is_empty() {
        eprintln!("Error: Could not detect input format for {}", input);
        std::process::exit(1);
    }
    let source_id = detected[0].0;

    // Get target format
    let target = registry.get(to).unwrap_or_else(|| {
        eprintln!(
            "Error: Unknown target format '{}'. Use --list-formats to see options.",
            to
        );
        std::process::exit(1);
    });

    // Parse
    let source = registry.get(source_id).unwrap();
    let resource = source.parser.parse(&content).unwrap_or_else(|e| {
        eprintln!("Error parsing {}: {e}", input);
        std::process::exit(1);
    });

    // Check data loss
    let warnings = check_data_loss(&resource, &target.writer.capabilities());
    if !warnings.is_empty() {
        eprintln!("Data loss warnings:");
        for w in &warnings {
            let icon = match w.severity {
                WarningSeverity::Error => "ERROR",
                WarningSeverity::Warning => "WARN",
                WarningSeverity::Info => "INFO",
            };
            eprintln!("  [{icon}] {}", w.message);
        }

        if cli.dry_run {
            return;
        }

        if !cli.force {
            eprint!("\nProceed? [y/N] ");
            let mut user_input = String::new();
            io::stdin().read_line(&mut user_input).unwrap();
            if !user_input.trim().eq_ignore_ascii_case("y") {
                eprintln!("Aborted.");
                std::process::exit(0);
            }
        }
    } else if cli.dry_run {
        eprintln!("No data loss warnings.");
        return;
    }

    // Write
    let output = target.writer.write(&resource).unwrap_or_else(|e| {
        eprintln!("Error writing output: {e}");
        std::process::exit(1);
    });

    match cli.out {
        Some(ref path) => {
            fs::write(path, &output).unwrap_or_else(|e| {
                eprintln!("Error writing to {path}: {e}");
                std::process::exit(1);
            });
            if cli.verbose {
                eprintln!(
                    "Converted {} ({}) -> {} ({})",
                    input, source_id, path, to
                );
                eprintln!("  Entries: {}", resource.entries.len());
                if !warnings.is_empty() {
                    eprintln!("  Warnings: {}", warnings.len());
                }
            }
        }
        None => {
            io::stdout().write_all(&output).unwrap();
            if cli.verbose {
                eprintln!("Converted {} ({}) -> stdout ({})", input, source_id, to);
                eprintln!("  Entries: {}", resource.entries.len());
                if !warnings.is_empty() {
                    eprintln!("  Warnings: {}", warnings.len());
                }
            }
        }
    }
}
