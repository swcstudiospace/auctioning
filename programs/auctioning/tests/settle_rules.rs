#![allow(clippy::assertions_on_constants, clippy::int_plus_one)]
use ::auctioning::{valid_ranking, Config, Race, RaceResult};
use anchor_lang::prelude::*;

fn r(entrant: Pubkey, rank: u16) -> RaceResult {
    RaceResult {
        entrant,
        score: 0,
        rank,
    }
}

#[test]
fn empty_results_are_a_valid_settlement() {
    assert!(valid_ranking(&[]));
}

#[test]
fn ranks_must_be_zero_based_and_contiguous() {
    let a = Pubkey::new_unique();
    let b = Pubkey::new_unique();
    let c = Pubkey::new_unique();
    assert!(valid_ranking(&[r(a, 0), r(b, 1), r(c, 2)]));
    assert!(!valid_ranking(&[r(a, 1), r(b, 2)]));
    assert!(!valid_ranking(&[r(a, 0), r(b, 2)]));
    assert!(!valid_ranking(&[r(a, 1), r(b, 0)]));
}

#[test]
fn duplicate_entrants_are_rejected() {
    let a = Pubkey::new_unique();
    assert!(!valid_ranking(&[r(a, 0), r(a, 1)]));
}

#[test]
fn max_results_still_fits_reserved_space() {
    let payload = 4 + Race::MAX_RESULTS * (32 + 8 + 2);
    assert!(Race::SPACE >= 8 + 32 + 8 + 32 + 8 + 8 + payload + 1 + 1);
}

#[test]
fn config_space_unchanged_by_pause_flag() {
    // `paused` took one byte from `reserved`; the account never needs realloc.
    assert_eq!(Config::SPACE, 8 + 32 + 32 + 2 + 1 + 64);
    assert_eq!(Config::MAX_FEE_BPS, 5000);
}
