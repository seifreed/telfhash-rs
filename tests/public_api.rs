use std::path::PathBuf;

use telfhash_rs::{
    GroupingMode, HashValue, NullDigestReason, TelfhashEngine, TelfhashOptions, TelfhashOutcome,
};

#[test]
fn library_api_hashes_and_groups_fixtures() {
    let engine = TelfhashEngine::new();
    let options = TelfhashOptions {
        grouping_mode: GroupingMode::Compatible,
        ..Default::default()
    };

    let results = engine
        .hash_paths(
            [
                PathBuf::from("tests/fixtures/bin/i386_dyn_stripped.so"),
                PathBuf::from("tests/fixtures/bin/x86_64_dyn_stripped.so"),
                PathBuf::from("tests/fixtures/bin/arm32_tnull.so"),
            ],
            &options,
        )
        .unwrap();

    assert!(matches!(
        results[0].outcome,
        TelfhashOutcome::Hash(HashValue::Digest(_))
    ));
    assert!(matches!(
        results[2].outcome,
        TelfhashOutcome::Hash(HashValue::NullDigest(
            NullDigestReason::InsufficientInformation
        ))
    ));

    let groups = engine.group(&results, &options).unwrap();
    assert_eq!(groups.grouped.len(), 1);
}
