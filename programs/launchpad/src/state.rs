use pinocchio::{error::ProgramError, AccountView, Address};

use crate::error::LaunchpadError;

pub const CONFIG_SEED: &[u8] = b"config";
pub const LAUNCH_SEED: &[u8] = b"launch";

/// Hard cap on the platform fee: 1 SOL.
pub const MAX_FEE_LAMPORTS: u64 = 1_000_000_000;

pub const CONFIG_DISC: u8 = 1;
pub const LAUNCH_DISC: u8 = 2;

/// Config layout: disc(1) | owner(32) | treasury(32) | fee_lamports(8 LE) | bump(1)
pub const CONFIG_LEN: usize = 1 + 32 + 32 + 8 + 1;
const CFG_OWNER: usize = 1;
const CFG_TREASURY: usize = 33;
const CFG_FEE: usize = 65;
const CFG_BUMP: usize = 73;

/// Launch layout: disc(1) | creator(32) | mint(32) | created_at(8 LE, unix) | bump(1)
pub const LAUNCH_LEN: usize = 1 + 32 + 32 + 8 + 1;
const LAUNCH_CREATOR: usize = 1;
const LAUNCH_MINT: usize = 33;
const LAUNCH_TIME: usize = 65;
const LAUNCH_BUMP: usize = 73;

pub struct Config {
    pub owner: [u8; 32],
    pub treasury: [u8; 32],
    pub fee_lamports: u64,
    pub bump: u8,
}

impl Config {
    pub fn write(&self, data: &mut [u8]) -> Result<(), ProgramError> {
        if data.len() != CONFIG_LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        data[0] = CONFIG_DISC;
        data[CFG_OWNER..CFG_OWNER + 32].copy_from_slice(&self.owner);
        data[CFG_TREASURY..CFG_TREASURY + 32].copy_from_slice(&self.treasury);
        data[CFG_FEE..CFG_FEE + 8].copy_from_slice(&self.fee_lamports.to_le_bytes());
        data[CFG_BUMP] = self.bump;
        Ok(())
    }

    pub fn read(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() != CONFIG_LEN || data[0] != CONFIG_DISC {
            return Err(LaunchpadError::InvalidConfig.into());
        }
        let mut owner = [0u8; 32];
        owner.copy_from_slice(&data[CFG_OWNER..CFG_OWNER + 32]);
        let mut treasury = [0u8; 32];
        treasury.copy_from_slice(&data[CFG_TREASURY..CFG_TREASURY + 32]);
        let mut fee = [0u8; 8];
        fee.copy_from_slice(&data[CFG_FEE..CFG_FEE + 8]);
        Ok(Self {
            owner,
            treasury,
            fee_lamports: u64::from_le_bytes(fee),
            bump: data[CFG_BUMP],
        })
    }

    pub fn set_fee(data: &mut [u8], fee_lamports: u64) {
        data[CFG_FEE..CFG_FEE + 8].copy_from_slice(&fee_lamports.to_le_bytes());
    }

    pub fn set_treasury(data: &mut [u8], treasury: &[u8; 32]) {
        data[CFG_TREASURY..CFG_TREASURY + 32].copy_from_slice(treasury);
    }
}

pub struct Launch {
    pub creator: [u8; 32],
    pub mint: [u8; 32],
    pub created_at: i64,
    pub bump: u8,
}

impl Launch {
    pub fn write(&self, data: &mut [u8]) -> Result<(), ProgramError> {
        if data.len() != LAUNCH_LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        data[0] = LAUNCH_DISC;
        data[LAUNCH_CREATOR..LAUNCH_CREATOR + 32].copy_from_slice(&self.creator);
        data[LAUNCH_MINT..LAUNCH_MINT + 32].copy_from_slice(&self.mint);
        data[LAUNCH_TIME..LAUNCH_TIME + 8].copy_from_slice(&self.created_at.to_le_bytes());
        data[LAUNCH_BUMP] = self.bump;
        Ok(())
    }
}

/// Canonical PDA lookup. Uses the syscall on-chain; the host build only needs to compile.
pub fn find_pda(seeds: &[&[u8]], program_id: &Address) -> (Address, u8) {
    #[cfg(any(target_os = "solana", target_arch = "bpf"))]
    {
        Address::find_program_address(seeds, program_id)
    }
    #[cfg(not(any(target_os = "solana", target_arch = "bpf")))]
    {
        let _ = (seeds, program_id);
        unreachable!("PDA derivation is only available on-chain")
    }
}

/// Loads and validates the config PDA: program-owned, canonical address, right layout.
pub fn load_config(program_id: &Address, account: &AccountView) -> Result<Config, ProgramError> {
    if !account.owned_by(program_id) {
        return Err(LaunchpadError::InvalidConfig.into());
    }
    let (expected, _) = crate::state::find_pda(&[CONFIG_SEED], program_id);
    if account.address() != &expected {
        return Err(LaunchpadError::InvalidPda.into());
    }
    let data = account.try_borrow()?;
    Config::read(&data)
}
