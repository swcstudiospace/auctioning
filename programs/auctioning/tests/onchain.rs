//! On-chain behaviour of the auctioning program under LiteSVM.
//!
//! Needs the SBF artefact: `cargo build-sbf` (from `programs/auctioning`)
//! produces `target/deploy/auctioning.so`. Without it the suite prints a
//! skip notice and passes, unless `REQUIRE_SBF=1` (CI sets it).
//!
//! Instruction bytes are built by hand (Anchor sighash + borsh) with
//! LiteSVM's own SDK crates, so this file also pins the wire format that
//! `backend/.../onchain.rs` and `magicblock/` produce.

use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

const SO_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/deploy/auctioning.so");
const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
const SYSTEM_PROGRAM: Address = Address::new_from_array([0; 32]);

fn program_id() -> Address {
    Address::new_from_array(::auctioning::ID.to_bytes())
}

fn sighash(name: &str) -> [u8; 8] {
    let h = Sha256::digest(format!("global:{name}").as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&h[..8]);
    out
}

fn pda(seeds: &[&[u8]]) -> Address {
    Address::find_program_address(seeds, &program_id()).0
}

fn borsh_string(s: &str) -> Vec<u8> {
    let mut v = (s.len() as u32).to_le_bytes().to_vec();
    v.extend_from_slice(s.as_bytes());
    v
}

struct Chain {
    svm: LiteSVM,
}

impl Chain {
    fn load() -> Option<Self> {
        let bytes = match std::fs::read(SO_PATH) {
            Ok(b) => b,
            Err(_) if std::env::var("REQUIRE_SBF").is_err() => {
                eprintln!("skipping: {SO_PATH} not built (run `cargo build-sbf`)");
                return None;
            }
            Err(e) => panic!("REQUIRE_SBF set but {SO_PATH} unreadable: {e}"),
        };
        let mut svm = LiteSVM::new();
        svm.add_program(program_id(), &bytes)
            .expect("load program");
        Some(Self { svm })
    }

    fn funded(&mut self) -> Keypair {
        let kp = Keypair::new();
        self.svm
            .airdrop(&kp.pubkey(), 100 * LAMPORTS_PER_SOL)
            .expect("airdrop");
        kp
    }

    fn send(&mut self, ix: Instruction, signers: &[&Keypair]) -> Result<(), String> {
        let payer = signers[0].pubkey();
        // Fresh blockhash per send so a retried identical message is not
        // rejected as AlreadyProcessed.
        self.svm.expire_blockhash();
        let msg = Message::new(&[ix], Some(&payer));
        let tx = Transaction::new(signers, msg, self.svm.latest_blockhash());
        match self.svm.send_transaction(tx) {
            Ok(_) => Ok(()),
            Err(failed) => Err(format!("{:?}\n{}", failed.err, failed.meta.logs.join("\n"))),
        }
    }

    fn data(&self, addr: &Address) -> Vec<u8> {
        self.svm
            .get_account(addr)
            .map(|a| a.data)
            .unwrap_or_default()
    }

    fn lamports(&self, addr: &Address) -> u64 {
        self.svm.get_account(addr).map(|a| a.lamports).unwrap_or(0)
    }

    // ---- instruction builders -------------------------------------------

    fn initialize(&self, authority: &Keypair, fee_vault: &Address, fee_bps: u16) -> Instruction {
        let mut data = sighash("initialize").to_vec();
        data.extend_from_slice(&fee_bps.to_le_bytes());
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(pda(&[b"config"]), false),
                AccountMeta::new_readonly(*fee_vault, false),
                AccountMeta::new(authority.pubkey(), true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            ],
            data,
        }
    }

    fn update_config(
        &self,
        authority: &Keypair,
        fee_bps: Option<u16>,
        paused: Option<bool>,
        new_authority: Option<Address>,
        new_fee_vault: Option<Address>,
    ) -> Instruction {
        let mut data = sighash("update_config").to_vec();
        match fee_bps {
            Some(b) => {
                data.push(1);
                data.extend_from_slice(&b.to_le_bytes());
            }
            None => data.push(0),
        }
        match paused {
            Some(p) => {
                data.push(1);
                data.push(p as u8);
            }
            None => data.push(0),
        }
        match new_authority {
            Some(a) => {
                data.push(1);
                data.extend_from_slice(a.as_ref());
            }
            None => data.push(0),
        }
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(pda(&[b"config"]), false),
                AccountMeta::new_readonly(authority.pubkey(), true),
                // Anchor optional account: the program id stands in for None.
                AccountMeta::new_readonly(new_fee_vault.unwrap_or_else(program_id), false),
            ],
            data,
        }
    }

    fn register_project(&self, owner: &Keypair, handle: &str) -> Instruction {
        let mut data = sighash("register_project").to_vec();
        data.extend(borsh_string(handle));
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(pda(&[b"project", owner.pubkey().as_ref()]), false),
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            ],
            data,
        }
    }

    fn log_paid_rp(
        &self,
        payer: &Keypair,
        project: &Address,
        fee_vault: &Address,
        seq: u64,
        rp: u64,
        lamports: u64,
        memo: &str,
    ) -> Instruction {
        let mut data = sighash("log_paid_rp").to_vec();
        data.extend_from_slice(&rp.to_le_bytes());
        data.extend_from_slice(&lamports.to_le_bytes());
        data.extend(borsh_string(memo));
        let receipt = pda(&[b"receipt", project.as_ref(), &seq.to_le_bytes()]);
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(*project, false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(*fee_vault, false),
                AccountMeta::new_readonly(pda(&[b"config"]), false),
                AccountMeta::new(receipt, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            ],
            data,
        }
    }

    fn open_race(&self, payer: &Keypair, project: &Address, nonce: u64) -> Instruction {
        let race = pda(&[b"race", project.as_ref(), &nonce.to_le_bytes()]);
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new_readonly(pda(&[b"config"]), false),
                AccountMeta::new(*project, false),
                AccountMeta::new(race, false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            ],
            data: sighash("open_race").to_vec(),
        }
    }

    fn settle_race(
        &self,
        settler: &Keypair,
        project: &Address,
        race_id: u64,
        results: &[(Address, u64, u16)],
    ) -> Instruction {
        let mut data = sighash("settle_race").to_vec();
        data.extend_from_slice(&(results.len() as u32).to_le_bytes());
        for (entrant, score, rank) in results {
            data.extend_from_slice(entrant.as_ref());
            data.extend_from_slice(&score.to_le_bytes());
            data.extend_from_slice(&rank.to_le_bytes());
        }
        let race = pda(&[b"race", project.as_ref(), &race_id.to_le_bytes()]);
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new_readonly(pda(&[b"config"]), false),
                AccountMeta::new(race, false),
                AccountMeta::new_readonly(settler.pubkey(), true),
            ],
            data,
        }
    }
}

// ---- account decoders --------------------------------------------------------

fn u64_at(d: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(d[at..at + 8].try_into().unwrap())
}

struct ConfigView {
    authority: [u8; 32],
    fee_vault: [u8; 32],
    fee_bps: u16,
    paused: bool,
}

fn config(d: &[u8]) -> ConfigView {
    ConfigView {
        authority: d[8..40].try_into().unwrap(),
        fee_vault: d[40..72].try_into().unwrap(),
        fee_bps: u16::from_le_bytes([d[72], d[73]]),
        paused: d[75] != 0,
    }
}

struct ProjectView {
    total_rp: u64,
    total_lamports: u64,
    race_nonce: u64,
    receipt_count: u64,
    handle: String,
}

fn project(d: &[u8]) -> ProjectView {
    let len = u32::from_le_bytes(d[72..76].try_into().unwrap()) as usize;
    ProjectView {
        total_rp: u64_at(d, 40),
        total_lamports: u64_at(d, 48),
        race_nonce: u64_at(d, 56),
        receipt_count: u64_at(d, 64),
        handle: String::from_utf8(d[76..76 + len].to_vec()).unwrap(),
    }
}

fn race_status_and_results(d: &[u8]) -> (u8, usize) {
    // 8 disc + 32 project + 8 race_id + 32 authority + 8 opened + 8 settled = 96
    let n = u32::from_le_bytes(d[96..100].try_into().unwrap()) as usize;
    let status = d[100 + n * 42];
    (status, n)
}

// ---- tests -------------------------------------------------------------------

#[test]
fn happy_path_initialize_register_pay_open_settle() {
    let Some(mut c) = Chain::load() else { return };
    let authority = c.funded();
    let vault = c.funded();
    let owner = c.funded();

    c.send(c.initialize(&authority, &vault.pubkey(), 300), &[&authority])
        .expect("initialize");
    let cfg = config(&c.data(&pda(&[b"config"])));
    assert_eq!(cfg.authority, authority.pubkey().to_bytes());
    assert_eq!(cfg.fee_vault, vault.pubkey().to_bytes());
    assert_eq!(cfg.fee_bps, 300);
    assert!(!cfg.paused);

    c.send(c.register_project(&owner, "beanz-coffee"), &[&owner])
        .expect("register");
    let project_pda = pda(&[b"project", owner.pubkey().as_ref()]);
    let p = project(&c.data(&project_pda));
    assert_eq!(p.handle, "beanz-coffee");
    assert_eq!((p.total_rp, p.race_nonce, p.receipt_count), (0, 0, 0));

    let vault_before = c.lamports(&vault.pubkey());
    c.send(
        c.log_paid_rp(&owner, &project_pda, &vault.pubkey(), 0, 10, LAMPORTS_PER_SOL, "whop:pay_1"),
        &[&owner],
    )
    .expect("log_paid_rp");
    let p = project(&c.data(&project_pda));
    assert_eq!(p.total_rp, 10);
    assert_eq!(p.total_lamports, LAMPORTS_PER_SOL);
    assert_eq!(p.receipt_count, 1);
    assert_eq!(c.lamports(&vault.pubkey()) - vault_before, LAMPORTS_PER_SOL);
    let receipt = pda(&[b"receipt", project_pda.as_ref(), &0u64.to_le_bytes()]);
    let r = c.data(&receipt);
    assert_eq!(&r[8..40], project_pda.as_ref());
    assert_eq!(u64_at(&r, 72), 10, "rp_amount");
    assert_eq!(u64_at(&r, 96), 0, "seq");

    // Second receipt uses the incremented counter.
    c.send(
        c.log_paid_rp(&owner, &project_pda, &vault.pubkey(), 1, 5, 1_000, ""),
        &[&owner],
    )
    .expect("second receipt");
    assert_eq!(project(&c.data(&project_pda)).receipt_count, 2);

    c.send(c.open_race(&owner, &project_pda, 0), &[&owner])
        .expect("open_race");
    assert_eq!(project(&c.data(&project_pda)).race_nonce, 1);
    let race = pda(&[b"race", project_pda.as_ref(), &0u64.to_le_bytes()]);
    assert_eq!(race_status_and_results(&c.data(&race)), (0, 0));

    let a = Keypair::new().pubkey();
    let b = Keypair::new().pubkey();
    c.send(
        c.settle_race(&owner, &project_pda, 0, &[(a, 900, 0), (b, 850, 1)]),
        &[&owner],
    )
    .expect("settle by race authority");
    assert_eq!(race_status_and_results(&c.data(&race)), (1, 2));

    let err = c
        .send(c.settle_race(&owner, &project_pda, 0, &[(a, 1, 0)]), &[&owner])
        .unwrap_err();
    assert!(err.contains("RaceAlreadySettled"), "{err}");
}

#[test]
fn guards_fee_vault_pause_authority_and_ranking() {
    let Some(mut c) = Chain::load() else { return };
    let authority = c.funded();
    let vault = c.funded();
    let owner = c.funded();
    let stranger = c.funded();

    // Fee out of range refuses.
    let err = c
        .send(c.initialize(&authority, &vault.pubkey(), 5001), &[&authority])
        .unwrap_err();
    assert!(err.contains("FeeOutOfRange"), "{err}");
    // Fee vault must be a system wallet (not the program itself).
    let err = c
        .send(c.initialize(&authority, &program_id(), 100), &[&authority])
        .unwrap_err();
    assert!(err.contains("BadFeeVault"), "{err}");

    c.send(c.initialize(&authority, &vault.pubkey(), 250), &[&authority])
        .unwrap();
    // Initialize is one-shot.
    assert!(c
        .send(c.initialize(&authority, &vault.pubkey(), 250), &[&authority])
        .is_err());

    c.send(c.register_project(&owner, "x"), &[&owner]).unwrap();
    let project_pda = pda(&[b"project", owner.pubkey().as_ref()]);

    // Wrong vault in log_paid_rp is refused before any transfer.
    let err = c
        .send(
            c.log_paid_rp(&owner, &project_pda, &stranger.pubkey(), 0, 1, 1_000, ""),
            &[&owner],
        )
        .unwrap_err();
    assert!(!err.is_empty());
    assert_eq!(project(&c.data(&project_pda)).receipt_count, 0);

    // A stranger cannot write receipts for someone else's project.
    let err = c
        .send(
            c.log_paid_rp(&stranger, &project_pda, &vault.pubkey(), 0, 1, 1_000, ""),
            &[&stranger],
        )
        .unwrap_err();
    assert!(err.contains("Unauthorized"), "{err}");

    // Only the authority may update config.
    let err = c
        .send(
            c.update_config(&stranger, None, Some(true), None, None),
            &[&stranger],
        )
        .unwrap_err();
    assert!(err.contains("Unauthorized"), "{err}");

    // Pause blocks purchases and race opens; settle stays allowed.
    c.send(
        c.update_config(&authority, Some(100), Some(true), None, None),
        &[&authority],
    )
    .unwrap();
    let cfg = config(&c.data(&pda(&[b"config"])));
    assert!(cfg.paused);
    assert_eq!(cfg.fee_bps, 100);
    let err = c
        .send(
            c.log_paid_rp(&owner, &project_pda, &vault.pubkey(), 0, 1, 1_000, ""),
            &[&owner],
        )
        .unwrap_err();
    assert!(err.contains("Paused"), "{err}");
    let err = c
        .send(c.open_race(&owner, &project_pda, 0), &[&owner])
        .unwrap_err();
    assert!(err.contains("Paused"), "{err}");

    // Unpause, rotate the vault, then a stranger may not open the owner's race.
    let vault2 = c.funded();
    c.send(
        c.update_config(&authority, None, Some(false), None, Some(vault2.pubkey())),
        &[&authority],
    )
    .unwrap();
    assert_eq!(
        config(&c.data(&pda(&[b"config"]))).fee_vault,
        vault2.pubkey().to_bytes()
    );
    let err = c
        .send(c.open_race(&stranger, &project_pda, 0), &[&stranger])
        .unwrap_err();
    assert!(err.contains("Unauthorized"), "{err}");
    c.send(c.open_race(&owner, &project_pda, 0), &[&owner]).unwrap();

    // Ranking must be canonical.
    let a = Keypair::new().pubkey();
    let b = Keypair::new().pubkey();
    for bad in [vec![(a, 1, 1), (b, 0, 0)], vec![(a, 1, 0), (a, 0, 1)], vec![(a, 1, 0), (b, 0, 2)]] {
        let err = c
            .send(c.settle_race(&owner, &project_pda, 0, &bad), &[&owner])
            .unwrap_err();
        assert!(err.contains("BadRanking"), "{err}");
    }
    // Strangers cannot settle; the protocol authority can.
    let err = c
        .send(c.settle_race(&stranger, &project_pda, 0, &[(a, 1, 0)]), &[&stranger])
        .unwrap_err();
    assert!(err.contains("Unauthorized"), "{err}");
    c.send(c.settle_race(&authority, &project_pda, 0, &[(a, 1, 0)]), &[&authority])
        .expect("protocol authority settles");

    // Authority rotation is irreversible for the old key.
    let multisig = c.funded();
    c.send(
        c.update_config(&authority, None, None, Some(multisig.pubkey()), None),
        &[&authority],
    )
    .unwrap();
    assert!(c
        .send(c.update_config(&authority, Some(1), None, None, None), &[&authority])
        .is_err());
    c.send(c.update_config(&multisig, Some(1), None, None, None), &[&multisig])
        .unwrap();
}
