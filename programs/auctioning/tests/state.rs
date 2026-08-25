use anchor_lang::prelude::*;
use ::auctioning::{Config, Project, Race, RaceResult};

#[test]
fn race_result_serializes_roundtrip() {
    let r = RaceResult {
        entrant: Pubkey::new_unique(),
        score: 12345,
        rank: 3,
    };
    let bytes = borsh::to_vec(&r).unwrap();
    assert_eq!(bytes.len(), 42); // 32 + 8 + 2
    let back: RaceResult = RaceResult::try_from_slice(&bytes).unwrap();
    assert_eq!(back.score, 12345);
    assert_eq!(back.rank, 3);
}

#[test]
fn spaces_are_stable() {
    // Guards against accidental space regressions that would break PDA init on upgrade.
    assert_eq!(Config::SPACE, 8 + 32 + 32 + 2 + 1 + 64);
    assert_eq!(Project::SPACE, 8 + 32 + 8 + 8 + 8 + 8 + (4 + 32) + 1);
    assert!(Race::SPACE > 8 + 32 + 8 + 32 + 8 + 8);
}
