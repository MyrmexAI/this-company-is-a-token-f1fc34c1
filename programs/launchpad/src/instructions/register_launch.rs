use pinocchio::{
    cpi::Seed,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::instructions::Transfer;

use super::{create_pda_account, require_signer, require_system_program, require_writable};
use crate::{
    error::LaunchpadError,
    state::{load_config, Launch, LAUNCH_LEN, LAUNCH_SEED},
};

const SPL_TOKEN_ID: Address =
    Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const TOKEN_2022_ID: Address =
    Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// Accounts: 0 creator (signer, writable) | 1 config PDA | 2 treasury (writable)
///           3 launch PDA (writable) | 4 mint | 5 system program
/// Data: none
pub fn process(program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [creator, config, treasury, launch, mint, system_program, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    if !data.is_empty() {
        return Err(LaunchpadError::InvalidInstructionData.into());
    }

    require_signer(creator)?;
    require_writable(creator)?;
    require_writable(treasury)?;
    require_writable(launch)?;
    require_system_program(system_program)?;

    let cfg = load_config(program_id, config)?;
    if treasury.address().as_array() != &cfg.treasury {
        return Err(LaunchpadError::InvalidTreasury.into());
    }
    check_mint(mint)?;

    let (expected, bump) =
        crate::state::find_pda(&[LAUNCH_SEED, mint.address().as_ref()], program_id);
    if launch.address() != &expected {
        return Err(LaunchpadError::InvalidPda.into());
    }
    if launch.owned_by(program_id) || launch.data_len() != 0 {
        return Err(LaunchpadError::AlreadyRegistered.into());
    }

    let created_at = Clock::get()?.unix_timestamp;

    let bump_seed = [bump];
    let seeds = [
        Seed::from(LAUNCH_SEED),
        Seed::from(mint.address().as_ref()),
        Seed::from(&bump_seed),
    ];
    create_pda_account(creator, launch, LAUNCH_LEN, program_id, &seeds)?;

    {
        let mut launch_data = launch.try_borrow_mut()?;
        Launch {
            creator: *creator.address().as_array(),
            mint: *mint.address().as_array(),
            created_at,
            bump,
        }
        .write(&mut launch_data)?;
    }

    if cfg.fee_lamports > 0 {
        Transfer {
            from: creator,
            to: treasury,
            lamports: cfg.fee_lamports,
        }
        .invoke()?;
    }
    Ok(())
}

/// The mint must be an initialized SPL Token or Token-2022 mint (not a token account or junk).
fn check_mint(mint: &AccountView) -> ProgramResult {
    if !(mint.owned_by(&SPL_TOKEN_ID) || mint.owned_by(&TOKEN_2022_ID)) {
        return Err(LaunchpadError::InvalidMint.into());
    }
    let data = mint.try_borrow()?;
    let len = data.len();
    let is_mint_layout = len == 82 || (len > 165 && data[165] == 1);
    if !is_mint_layout || data[45] != 1 {
        return Err(LaunchpadError::InvalidMint.into());
    }
    Ok(())
}
