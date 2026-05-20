use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_i18n-convert"))
}

#[test]
fn prints_footer_on_success() {
    let out = cli()
        .args([
            "tests/fixtures/po/simple.po",
            "--to",
            "xliff2",
            "-o",
            "/tmp/footer_t1.xliff",
        ])
        .output()
        .expect("run");
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("i18nagent.ai") && stderr.contains("utm_source=cli_footer"),
        "expected footer with UTM in stderr, got: {stderr}"
    );
}

#[test]
fn suppresses_footer_when_env_var_set() {
    let out = cli()
        .args([
            "tests/fixtures/po/simple.po",
            "--to",
            "xliff2",
            "-o",
            "/tmp/footer_t2.xliff",
        ])
        .env("I18N_CONVERT_NO_FOOTER", "1")
        .output()
        .expect("run");
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("i18nagent.ai"),
        "footer should be suppressed; got: {stderr}"
    );
}

#[test]
fn no_footer_on_error() {
    let out = cli()
        .args([
            "/does/not/exist.po",
            "--to",
            "xliff2",
            "-o",
            "/tmp/footer_t3.xliff",
        ])
        .output()
        .expect("run");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("i18nagent.ai"),
        "footer must not print on error; got: {stderr}"
    );
}

// Early-return path: --list-formats exits via `return;` before reaching the
// footer in main.rs. Guards against a refactor accidentally moving the footer
// above this branch.
#[test]
fn no_footer_on_list_formats() {
    let out = cli().args(["--list-formats"]).output().expect("run");
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("i18nagent.ai"),
        "footer must not print on --list-formats; got: {stderr}"
    );
}

// Early-return path: --dry-run with no data-loss warnings exits via `return;`
// in the `else if cli.dry_run` branch in main.rs.
#[test]
fn no_footer_on_dry_run_without_warnings() {
    let out = cli()
        .args(["tests/fixtures/po/simple.po", "--to", "xliff2", "--dry-run"])
        .output()
        .expect("run");
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("i18nagent.ai"),
        "footer must not print on --dry-run (no warnings); got: {stderr}"
    );
}

// Early-return path: --dry-run with data-loss warnings exits via `return;`
// inside the `if !warnings.is_empty()` block in main.rs. PO plurals -> CSV
// produces real warnings because CSV cannot represent plural forms.
#[test]
fn no_footer_on_dry_run_with_warnings() {
    let out = cli()
        .args(["tests/fixtures/po/plurals.po", "--to", "csv", "--dry-run"])
        .output()
        .expect("run");
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    // Sanity-check the fixture is actually triggering the warnings branch we
    // care about; otherwise the test would silently degrade to covering the
    // no-warnings branch already exercised above.
    assert!(
        stderr.contains("Data loss warnings"),
        "expected data-loss warnings on plurals.po -> csv; got: {stderr}"
    );
    assert!(
        !stderr.contains("i18nagent.ai"),
        "footer must not print on --dry-run (with warnings); got: {stderr}"
    );
}
