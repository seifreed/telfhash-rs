use std::fs;
use std::path::Path;

use assert_cmd::Command;

fn repo_root() -> &'static str {
    env!("CARGO_MANIFEST_DIR")
}

fn read(path: &str) -> String {
    fs::read_to_string(Path::new(repo_root()).join(path)).unwrap()
}

fn bin() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("telfhash"))
}

#[test]
fn plain_output_matches_golden() {
    bin()
        .current_dir(repo_root())
        .args(["tests/fixtures/bin/*"])
        .assert()
        .success()
        .stdout(read("tests/golden/hash_plain.txt"));
}

#[test]
fn tsv_output_matches_golden() {
    bin()
        .current_dir(repo_root())
        .args(["-f", "tsv", "tests/fixtures/bin/*"])
        .assert()
        .success()
        .stdout(read("tests/golden/hash.tsv"));
}

#[test]
fn json_output_matches_golden() {
    bin()
        .current_dir(repo_root())
        .args(["-f", "json", "tests/fixtures/bin/*"])
        .assert()
        .success()
        .stdout(read("tests/golden/hash.json"));
}

#[test]
fn sarif_output_matches_golden() {
    bin()
        .current_dir(repo_root())
        .args(["-f", "sarif", "tests/fixtures/bin/*"])
        .assert()
        .success()
        .stdout(read("tests/golden/hash.sarif"));
}

#[test]
fn compatible_group_output_matches_golden() {
    bin()
        .current_dir(repo_root())
        .args(["-g", "tests/fixtures/bin/*"])
        .assert()
        .success()
        .stdout(read("tests/golden/group_compatible.txt"));
}

#[test]
fn connected_components_group_output_matches_golden() {
    bin()
        .current_dir(repo_root())
        .args([
            "-g",
            "--group-mode",
            "connected-components",
            "tests/fixtures/bin/*",
        ])
        .assert()
        .success()
        .stdout(read("tests/golden/group_connected.txt"));
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
    bin()
        .current_dir(repo_root())
        .args(["-f", "json", "tests/fixtures/bin/not_elf_archive.a"])
        .assert()
        .success()
        .stdout("[{\"file\":\"tests/fixtures/bin/not_elf_archive.a\",\"telfhash\":\"-\",\"msg\":\"Could not parse file as ELF\"}]\n");
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
