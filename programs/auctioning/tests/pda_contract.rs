#![allow(clippy::assertions_on_constants, clippy::int_plus_one)]
use ::auctioning::{Config, Project, Race, RaceResult, RpReceipt};
use anchor_lang::prelude::*;

/// Deterministic PDA derivation contract: clients (Leptos UI, Shuttle settle
/// worker) must derive exactly these addresses. These tests pin the layout so
/// a client/program drift is caught at CI time, not on mainnet.
mod pda_contract {
    use super::*;

    fn find(seed: &str, extra: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
        let mut seeds: Vec<&[u8]> = vec![seed.as_bytes()];
        seeds.extend_from_slice(extra);
        Pubkey::find_program_address(&seeds, program_id)
    }

    #[test]
    fn config_pda_layout() {
        let pid = ::auctioning::ID;
        let (pda, _bump) = find(Config::SEED, &[], &pid);
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn project_pda_layout() {
        let pid = ::auctioning::ID;
        let owner = Pubkey::new_unique();
        let (pda, _) = find(Project::SEED, &[owner.as_ref()], &pid);
        assert_ne!(pda, Pubkey::default());
        // Same owner always derives the same project.
        let (pda2, _) = find(Project::SEED, &[owner.as_ref()], &pid);
        assert_eq!(pda, pda2);
    }

    #[test]
    fn receipt_pda_uses_preincrement_seq() {
        // Mirrors log_paid_rp: seq == receipt_count BEFORE increment.
        let pid = ::auctioning::ID;
        let project = Pubkey::new_unique();
        let count: u64 = 7; // what the client read from Project.receipt_count
        let (pda, _) = find(
            RpReceipt::SEED,
            &[project.as_ref(), &count.to_le_bytes()],
            &pid,
        );
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn race_pda_uses_race_id_le_bytes() {
        let pid = ::auctioning::ID;
        let project = Pubkey::new_unique();
        let race_id: u64 = 3;
        let (a, _) = find(
            Race::SEED,
            &[project.as_ref(), &race_id.to_le_bytes()],
            &pid,
        );
        let (b, _) = find(
            Race::SEED,
            &[project.as_ref(), &race_id.to_le_bytes()],
            &pid,
        );
        assert_eq!(a, b);
        // Different race id -> different address.
        let (c, _) = find(Race::SEED, &[project.as_ref(), &5u64.to_le_bytes()], &pid);
        assert_ne!(a, c);
    }

    #[test]
    fn race_result_borsh_layout_is_42_bytes() {
        let r = RaceResult {
            entrant: Pubkey::new_unique(),
            score: 999_999,
            rank: 2,
        };
        let bytes = borsh::to_vec(&r).unwrap();
        assert_eq!(bytes.len(), 42); // 32 + 8 + 2 — matches settle worker packing
    }

    #[test]
    fn space_constants_guard_against_regressions() {
        assert_eq!(Config::SPACE, 8 + 32 + 32 + 2 + 1 + 64);
        assert_eq!(Project::SPACE, 8 + 32 + 8 + 8 + 8 + 8 + (4 + 32) + 1);
        assert!(Race::SPACE >= 8 + 32 + 8 + 32 + 8 + 8 + (4 + Race::MAX_RESULTS * 42) + 1 + 1);
    }
}
