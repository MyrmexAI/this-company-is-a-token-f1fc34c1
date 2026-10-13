//! Integration tests (LiteSVM). Build the program first: `cargo build-sbf`, then `cargo test`.

use {
    antpad_launchpad::{
        error::LaunchpadError, ID, IX_INIT, IX_REGISTER_LAUNCH, IX_SET_FEE, IX_SET_TREASURY,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    solana_account::Account,
    solana_address::Address,
    solana_clock::Clock,
    solana_instruction::{AccountMeta, Instruction},
    solana_instruction_error::InstructionError,
    solana_keypair::Keypair,
    solana_message::Message,
    solana_signer::Signer,
    solana_transaction::Transaction,
    solana_transaction_error::TransactionError,
};

const SOL: u64 = 1_000_000_000;
const FEE: u64 = 50_000_000; // 0.05 SOL
const NOW: i64 = 1_760_000_000;
const SPL_TOKEN: Address = Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const TOKEN_2022: Address = Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const SYSTEM: Address = Address::from_str_const("11111111111111111111111111111111");

struct Env {
    svm: LiteSVM,
    owner: Keypair,
    treasury: Address,
    config: Address,
}

fn program_path() -> String {
    format!(
        "{}/../../target/deploy/antpad_launchpad.so",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn config_pda() -> Address {
    Address::find_program_address(&[b"config"], &ID).0
}

fn launch_pda(mint: &Address) -> Address {
    Address::find_program_address(&[b"launch", mint.as_ref()], &ID).0
}

fn new_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(ID, program_path())
        .expect("program .so missing: run `cargo build-sbf` first");
    svm.set_sysvar(&Clock {
        unix_timestamp: NOW,
        ..Default::default()
    });
    svm
}

fn send(svm: &mut LiteSVM, payer: &Keypair, signers: &[&Keypair], ix: Instruction) -> TransactionResult {
    let msg = Message::new(&[ix], Some(&payer.pubkey()));
    let mut all: Vec<&Keypair> = vec![payer];
    all.extend_from_slice(signers);
    let tx = Transaction::new(&all, msg, svm.latest_blockhash());
    let res = svm.send_transaction(tx);
    svm.expire_blockhash();
    res
}

fn ix_init(payer: &Address, owner: &Address, treasury: &Address, fee: u64) -> Instruction {
    let mut data = vec![IX_INIT];
    data.extend_from_slice(treasury.as_ref());
    data.extend_from_slice(&fee.to_le_bytes());
    Instruction::new_with_bytes(
        ID,
        &data,
        vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new(config_pda(), false),
            AccountMeta::new_readonly(SYSTEM, false),
        ],
    )
}

fn ix_set_fee(owner: &Address, fee: u64) -> Instruction {
    let mut data = vec![IX_SET_FEE];
    data.extend_from_slice(&fee.to_le_bytes());
    Instruction::new_with_bytes(
        ID,
        &data,
        vec![
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new(config_pda(), false),
        ],
    )
}

fn ix_set_treasury(owner: &Address, treasury: &Address) -> Instruction {
    let mut data = vec![IX_SET_TREASURY];
    data.extend_from_slice(treasury.as_ref());
    Instruction::new_with_bytes(
        ID,
        &data,
        vec![
            AccountMeta::new_readonly(*owner, true),
            AccountMeta::new(config_pda(), false),
        ],
    )
}

fn ix_register(creator: &Address, treasury: &Address, mint: &Address, creator_signs: bool) -> Instruction {
    Instruction::new_with_bytes(
        ID,
        &[IX_REGISTER_LAUNCH],
        vec![
            AccountMeta::new(*creator, creator_signs),
            AccountMeta::new_readonly(config_pda(), false),
            AccountMeta::new(*treasury, false),
            AccountMeta::new(launch_pda(mint), false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(SYSTEM, false),
        ],
    )
}

fn mint_account(owner: Address) -> Account {
    let mut data = vec![0u8; 82];
    data[45] = 1; // is_initialized
    Account {
        lamports: 1_461_600,
        data,
        owner,
        executable: false,
        rent_epoch: 0,
    }
}

fn add_mint(svm: &mut LiteSVM) -> Address {
    let mint = Address::new_unique();
    svm.set_account(mint, mint_account(SPL_TOKEN)).unwrap();
    mint
}

fn assert_custom(res: TransactionResult, expected: LaunchpadError) {
    match res {
        Ok(_) => panic!("expected {expected:?}, but the transaction succeeded"),
        Err(e) => assert_eq!(
            e.err,
            TransactionError::InstructionError(0, InstructionError::Custom(expected as u32)),
            "logs: {:?}",
            e.meta.logs
        ),
    }
}

fn setup() -> Env {
    let mut svm = new_svm();
    let owner = Keypair::new();
    svm.airdrop(&owner.pubkey(), 10 * SOL).unwrap();
    let treasury = Address::new_unique();
    let ix = ix_init(&owner.pubkey(), &owner.pubkey(), &treasury, FEE);
    send(&mut svm, &owner, &[], ix).expect("init failed");
    Env {
        svm,
        owner,
        treasury,
        config: config_pda(),
    }
}

fn read_config(env: &Env) -> (Address, Address, u64) {
    let data = env.svm.get_account(&env.config).unwrap().data;
    assert_eq!(data[0], 1);
    (
        Address::try_from(&data[1..33]).unwrap(),
        Address::try_from(&data[33..65]).unwrap(),
        u64::from_le_bytes(data[65..73].try_into().unwrap()),
    )
}

fn new_creator(env: &mut Env) -> Keypair {
    let kp = Keypair::new();
    env.svm.airdrop(&kp.pubkey(), 5 * SOL).unwrap();
    kp
}

#[test]
fn init_creates_config() {
    let env = setup();
    let acc = env.svm.get_account(&env.config).unwrap();
    assert_eq!(acc.owner, ID);
    let (owner, treasury, fee) = read_config(&env);
    assert_eq!(owner, env.owner.pubkey());
    assert_eq!(treasury, env.treasury);
    assert_eq!(fee, FEE);
}

#[test]
fn init_twice_fails() {
    let mut env = setup();
    let attacker = new_creator(&mut env);
    let ix = ix_init(&attacker.pubkey(), &attacker.pubkey(), &attacker.pubkey(), 0);
    assert_custom(
        send(&mut env.svm, &attacker, &[], ix),
        LaunchpadError::ConfigAlreadyInitialized,
    );
    assert_eq!(read_config(&env).0, env.owner.pubkey());
}

#[test]
fn init_requires_owner_signature() {
    let mut svm = new_svm();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), SOL).unwrap();
    let not_signing_owner = Address::new_unique();
    let mut ix = ix_init(&payer.pubkey(), &not_signing_owner, &Address::new_unique(), FEE);
    ix.accounts[1].is_signer = false;
    assert!(send(&mut svm, &payer, &[], ix).is_err());
    assert!(svm.get_account(&config_pda()).is_none());
}

#[test]
fn init_fee_above_cap_fails_and_cap_is_allowed() {
    let mut svm = new_svm();
    let owner = Keypair::new();
    svm.airdrop(&owner.pubkey(), SOL).unwrap();
    let treasury = Address::new_unique();

    let ix = ix_init(&owner.pubkey(), &owner.pubkey(), &treasury, SOL + 1);
    assert_custom(send(&mut svm, &owner, &[], ix), LaunchpadError::FeeAboveCap);
    assert!(svm.get_account(&config_pda()).is_none());

    let ix = ix_init(&owner.pubkey(), &owner.pubkey(), &treasury, SOL);
    send(&mut svm, &owner, &[], ix).expect("fee == cap must be accepted");
}

#[test]
fn init_works_when_config_pda_was_prefunded() {
    let mut svm = new_svm();
    let owner = Keypair::new();
    svm.airdrop(&owner.pubkey(), SOL).unwrap();
    svm.airdrop(&config_pda(), 1_000_000).unwrap();
    let ix = ix_init(&owner.pubkey(), &owner.pubkey(), &Address::new_unique(), FEE);
    send(&mut svm, &owner, &[], ix).expect("griefing pre-fund must not block init");
    assert_eq!(svm.get_account(&config_pda()).unwrap().owner, ID);
}

#[test]
fn register_launch_pays_fee_to_treasury_and_records() {
    let mut env = setup();
    let creator = new_creator(&mut env);
    let mint = add_mint(&mut env.svm);
    let before = env.svm.get_balance(&creator.pubkey()).unwrap();
    assert!(env.svm.get_balance(&env.treasury).is_none());

    let ix = ix_register(&creator.pubkey(), &env.treasury, &mint, true);
    send(&mut env.svm, &creator, &[], ix).expect("register failed");

    assert_eq!(env.svm.get_balance(&env.treasury).unwrap(), FEE);
    let launch = env.svm.get_account(&launch_pda(&mint)).unwrap();
    assert_eq!(launch.owner, ID);
    assert_eq!(launch.data[0], 2);
    assert_eq!(&launch.data[1..33], creator.pubkey().as_ref());
    assert_eq!(&launch.data[33..65], mint.as_ref());
    assert_eq!(i64::from_le_bytes(launch.data[65..73].try_into().unwrap()), NOW);

    let rent = env.svm.minimum_balance_for_rent_exemption(launch.data.len());
    assert_eq!(launch.lamports, rent);
    let after = env.svm.get_balance(&creator.pubkey()).unwrap();
    assert_eq!(before - after, FEE + rent + 5000);
}

#[test]
fn register_works_for_token_2022_mint() {
    let mut env = setup();
    let creator = new_creator(&mut env);
    let mint = Address::new_unique();
    env.svm.set_account(mint, mint_account(TOKEN_2022)).unwrap();
    let ix = ix_register(&creator.pubkey(), &env.treasury, &mint, true);
    send(&mut env.svm, &creator, &[], ix).expect("register failed");
    assert_eq!(env.svm.get_balance(&env.treasury).unwrap(), FEE);
}

#[test]
fn register_same_mint_twice_fails() {
    let mut env = setup();
    let creator = new_creator(&mut env);
    let other = new_creator(&mut env);
    let mint = add_mint(&mut env.svm);

    let ix = ix_register(&creator.pubkey(), &env.treasury, &mint, true);
    send(&mut env.svm, &creator, &[], ix).unwrap();
    let treasury_after_first = env.svm.get_balance(&env.treasury).unwrap();

    let ix = ix_register(&creator.pubkey(), &env.treasury, &mint, true);
    assert_custom(send(&mut env.svm, &creator, &[], ix), LaunchpadError::AlreadyRegistered);
    let ix = ix_register(&other.pubkey(), &env.treasury, &mint, true);
    assert_custom(send(&mut env.svm, &other, &[], ix), LaunchpadError::AlreadyRegistered);

    assert_eq!(env.svm.get_balance(&env.treasury).unwrap(), treasury_after_first);
    let launch = env.svm.get_account(&launch_pda(&mint)).unwrap();
    assert_eq!(&launch.data[1..33], creator.pubkey().as_ref());
}

#[test]
fn register_works_when_launch_pda_was_prefunded() {
    let mut env = setup();
    let creator = new_creator(&mut env);
    let mint = add_mint(&mut env.svm);
    env.svm.airdrop(&launch_pda(&mint), SOL).unwrap();
    let ix = ix_register(&creator.pubkey(), &env.treasury, &mint, true);
    send(&mut env.svm, &creator, &[], ix).expect("griefing pre-fund must not block registration");
    assert_eq!(env.svm.get_account(&launch_pda(&mint)).unwrap().owner, ID);
}

#[test]
fn register_requires_creator_signature() {
    let mut env = setup();
    let victim = new_creator(&mut env);
    let payer = new_creator(&mut env);
    let mint = add_mint(&mut env.svm);
    let ix = ix_register(&victim.pubkey(), &env.treasury, &mint, false);
    assert!(send(&mut env.svm, &payer, &[], ix).is_err());
    assert!(env.svm.get_account(&launch_pda(&mint)).is_none());
}

#[test]
fn register_rejects_wrong_treasury() {
    let mut env = setup();
    let creator = new_creator(&mut env);
    let mint = add_mint(&mut env.svm);
    let ix = ix_register(&creator.pubkey(), &Address::new_unique(), &mint, true);
    assert_custom(send(&mut env.svm, &creator, &[], ix), LaunchpadError::InvalidTreasury);
}

#[test]
fn register_rejects_non_mint_account() {
    let mut env = setup();
    let creator = new_creator(&mut env);

    // Plain wallet.
    let ix = ix_register(&creator.pubkey(), &env.treasury, &Address::new_unique(), true);
    assert_custom(send(&mut env.svm, &creator, &[], ix), LaunchpadError::InvalidMint);

    // Token account (165 bytes) owned by the token program.
    let token_acc = Address::new_unique();
    let mut acc = mint_account(SPL_TOKEN);
    acc.data = vec![1u8; 165];
    env.svm.set_account(token_acc, acc).unwrap();
    let ix = ix_register(&creator.pubkey(), &env.treasury, &token_acc, true);
    assert_custom(send(&mut env.svm, &creator, &[], ix), LaunchpadError::InvalidMint);

    // Uninitialized mint.
    let blank = Address::new_unique();
    let mut acc = mint_account(SPL_TOKEN);
    acc.data[45] = 0;
    env.svm.set_account(blank, acc).unwrap();
    let ix = ix_register(&creator.pubkey(), &env.treasury, &blank, true);
    assert_custom(send(&mut env.svm, &creator, &[], ix), LaunchpadError::InvalidMint);
}

#[test]
fn register_rejects_forged_config() {
    let mut env = setup();
    let creator = new_creator(&mut env);
    let mint = add_mint(&mut env.svm);
    let fake_config = Address::new_unique();
    let mut data = vec![0u8; 74];
    data[0] = 1;
    data[33..65].copy_from_slice(creator.pubkey().as_ref());
    env.svm
        .set_account(fake_config, Account { lamports: SOL, data, owner: ID, executable: false, rent_epoch: 0 })
        .unwrap();
    let mut ix = ix_register(&creator.pubkey(), &creator.pubkey(), &mint, true);
    ix.accounts[1].pubkey = fake_config;
    assert_custom(send(&mut env.svm, &creator, &[], ix), LaunchpadError::InvalidPda);
}

#[test]
fn owner_can_change_fee_and_new_fee_is_charged() {
    let mut env = setup();
    let ix = ix_set_fee(&env.owner.pubkey(), 2 * FEE);
    let owner = env.owner.insecure_clone();
    send(&mut env.svm, &owner, &[], ix).expect("set_fee failed");
    assert_eq!(read_config(&env).2, 2 * FEE);

    let creator = new_creator(&mut env);
    let mint = add_mint(&mut env.svm);
    let ix = ix_register(&creator.pubkey(), &env.treasury, &mint, true);
    send(&mut env.svm, &creator, &[], ix).unwrap();
    assert_eq!(env.svm.get_balance(&env.treasury).unwrap(), 2 * FEE);

    // Fee 0 is allowed: registration is free and no transfer is attempted.
    let ix = ix_set_fee(&owner.pubkey(), 0);
    send(&mut env.svm, &owner, &[], ix).unwrap();
    let mint2 = add_mint(&mut env.svm);
    let ix = ix_register(&creator.pubkey(), &env.treasury, &mint2, true);
    send(&mut env.svm, &creator, &[], ix).unwrap();
    assert_eq!(env.svm.get_balance(&env.treasury).unwrap(), 2 * FEE);
}

#[test]
fn set_fee_above_cap_fails() {
    let mut env = setup();
    let owner = env.owner.insecure_clone();
    let ix = ix_set_fee(&owner.pubkey(), SOL + 1);
    assert_custom(send(&mut env.svm, &owner, &[], ix), LaunchpadError::FeeAboveCap);
    assert_eq!(read_config(&env).2, FEE);

    let ix = ix_set_fee(&owner.pubkey(), SOL);
    send(&mut env.svm, &owner, &[], ix).expect("cap itself is allowed");
    assert_eq!(read_config(&env).2, SOL);
}

#[test]
fn stranger_cannot_change_fee() {
    let mut env = setup();
    let stranger = new_creator(&mut env);
    let ix = ix_set_fee(&stranger.pubkey(), 0);
    assert_custom(send(&mut env.svm, &stranger, &[], ix), LaunchpadError::NotOwner);
    assert_eq!(read_config(&env).2, FEE);
}

#[test]
fn stranger_cannot_change_treasury() {
    let mut env = setup();
    let stranger = new_creator(&mut env);
    let ix = ix_set_treasury(&stranger.pubkey(), &stranger.pubkey());
    assert_custom(send(&mut env.svm, &stranger, &[], ix), LaunchpadError::NotOwner);
    assert_eq!(read_config(&env).1, env.treasury);
}

#[test]
fn owner_signature_is_required_even_if_owner_key_is_passed() {
    let mut env = setup();
    let stranger = new_creator(&mut env);
    let mut ix = ix_set_fee(&env.owner.pubkey(), 0);
    ix.accounts[0].is_signer = false;
    assert!(send(&mut env.svm, &stranger, &[], ix).is_err());
    let mut ix = ix_set_treasury(&env.owner.pubkey(), &stranger.pubkey());
    ix.accounts[0].is_signer = false;
    assert!(send(&mut env.svm, &stranger, &[], ix).is_err());
    assert_eq!(read_config(&env).2, FEE);
    assert_eq!(read_config(&env).1, env.treasury);
}

#[test]
fn owner_can_change_treasury_and_fees_follow() {
    let mut env = setup();
    let owner = env.owner.insecure_clone();
    let new_treasury = Address::new_unique();
    let ix = ix_set_treasury(&owner.pubkey(), &new_treasury);
    send(&mut env.svm, &owner, &[], ix).expect("set_treasury failed");
    assert_eq!(read_config(&env).1, new_treasury);

    let creator = new_creator(&mut env);
    let mint = add_mint(&mut env.svm);
    // Old treasury is no longer accepted.
    let ix = ix_register(&creator.pubkey(), &env.treasury, &mint, true);
    assert_custom(send(&mut env.svm, &creator, &[], ix), LaunchpadError::InvalidTreasury);
    let ix = ix_register(&creator.pubkey(), &new_treasury, &mint, true);
    send(&mut env.svm, &creator, &[], ix).unwrap();
    assert_eq!(env.svm.get_balance(&new_treasury).unwrap(), FEE);
    assert!(env.svm.get_balance(&env.treasury).is_none());
}

#[test]
fn zero_treasury_is_rejected() {
    let mut env = setup();
    let owner = env.owner.insecure_clone();
    let ix = ix_set_treasury(&owner.pubkey(), &SYSTEM);
    assert_custom(send(&mut env.svm, &owner, &[], ix), LaunchpadError::InvalidTreasury);
}

#[test]
fn unknown_instruction_and_bad_data_fail() {
    let mut env = setup();
    let owner = env.owner.insecure_clone();
    let ix = Instruction::new_with_bytes(ID, &[99], vec![]);
    assert_custom(send(&mut env.svm, &owner, &[], ix), LaunchpadError::InvalidInstructionData);
    let ix = Instruction::new_with_bytes(
        ID,
        &[IX_SET_FEE, 1, 2, 3],
        vec![
            AccountMeta::new_readonly(owner.pubkey(), true),
            AccountMeta::new(env.config, false),
        ],
    );
    assert_custom(send(&mut env.svm, &owner, &[], ix), LaunchpadError::InvalidInstructionData);
}
