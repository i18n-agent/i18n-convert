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
