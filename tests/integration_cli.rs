use std::fs;
use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;

fn repo_root() -> &'static str {
    env!("CARGO_MANIFEST_DIR")
}

fn read(path: &str) -> String {
    fs::read_to_string(Path::new(repo_root()).join(path)).unwrap()
}

fn bin() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("telfhash"))
}

fn normalize_line_endings(value: &str) -> String {
    value.replace("\r\n", "\n")
}

fn stdout_for(args: &[&str]) -> String {
    let assert = bin().current_dir(repo_root()).args(args).assert().success();
    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

fn assert_text_output(args: &[&str], golden_path: &str) {
    let actual = normalize_line_endings(&stdout_for(args));
    let expected = normalize_line_endings(&read(golden_path));
    assert_eq!(actual, expected);
}

fn assert_json_output(args: &[&str], golden_path: &str) {
    let actual: Value = serde_json::from_str(&stdout_for(args)).unwrap();
    let expected: Value = serde_json::from_str(&read(golden_path)).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn plain_output_matches_golden() {
    assert_text_output(&["tests/fixtures/bin/*"], "tests/golden/hash_plain.txt");
}

#[test]
fn tsv_output_matches_golden() {
    assert_text_output(
        &["-f", "tsv", "tests/fixtures/bin/*"],
        "tests/golden/hash.tsv",
    );
}

#[test]
fn json_output_matches_golden() {
    assert_json_output(
        &["-f", "json", "tests/fixtures/bin/*"],
        "tests/golden/hash.json",
    );
}

#[test]
fn sarif_output_matches_golden() {
    assert_json_output(
        &["-f", "sarif", "tests/fixtures/bin/*"],
        "tests/golden/hash.sarif",
    );
}

#[test]
fn compatible_group_output_matches_golden() {
    assert_text_output(
        &["-g", "tests/fixtures/bin/*"],
        "tests/golden/group_compatible.txt",
    );
}

#[test]
fn connected_components_group_output_matches_golden() {
    assert_text_output(
        &[
            "-g",
            "--group-mode",
            "connected-components",
            "tests/fixtures/bin/*",
        ],
        "tests/golden/group_connected.txt",
    );
}

#[test]
fn debug_output_reports_key_metadata() {
    bin()
        .current_dir(repo_root())
        .args(["-d", "tests/fixtures/bin/arm32_tnull.so"])
        .assert()
        .success()
        .stderr(predicates::str::contains("ELFCLASS32"))
        .stderr(predicates::str::contains("symbol table: SHT_DYNSYM"))
        .stderr(predicates::str::contains("symbols found"))
        .stderr(predicates::str::contains("symbols considered"))
        .stderr(predicates::str::contains("TLSH result: tnull"));
}

#[test]
fn debug_output_reports_no_symbol_reason() {
    bin()
        .current_dir(repo_root())
        .args(["-d", "tests/fixtures/bin/x86_64_static_pie_stripped"])
        .assert()
        .success()
        .stderr(predicates::str::contains("result: - (no symbols)"));
}

#[test]
fn invalid_elf_keeps_legacy_message() {
    let actual: Value = serde_json::from_str(&stdout_for(&[
        "-f",
        "json",
        "tests/fixtures/bin/not_elf_archive.a",
    ]))
    .unwrap();

    assert_eq!(
        actual,
        serde_json::json!([{
            "file":"tests/fixtures/bin/not_elf_archive.a",
            "telfhash":"-",
            "msg":"Could not parse file as ELF"
        }])
    );
}

#[test]
fn invalid_elf_is_reported_in_sarif() {
    bin()
        .current_dir(repo_root())
        .args(["-f", "sarif", "tests/fixtures/bin/not_elf_archive.a"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"ruleId\":\"TFL005\""))
        .stdout(predicates::str::contains(
            "\"msg\":\"Could not parse file as ELF\"",
        ));
}
