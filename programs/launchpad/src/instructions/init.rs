use pinocchio::{cpi::Seed, AccountView, Address, ProgramResult};

use super::{create_pda_account, require_signer, require_system_program, require_writable};
use crate::{
    error::LaunchpadError,
    state::{Config, CONFIG_LEN, CONFIG_SEED, MAX_FEE_LAMPORTS},
};

/// Accounts: 0 payer (signer, writable) | 1 owner (signer) | 2 config PDA (writable) | 3 system program
/// Data: treasury (32) | fee_lamports (8, LE)
pub fn process(program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [payer, owner, config, system_program, ..] = accounts else {
        return Err(pinocchio::error::ProgramError::NotEnoughAccountKeys);
    };
    if data.len() != 40 {
        return Err(LaunchpadError::InvalidInstructionData.into());
    }
    let mut treasury = [0u8; 32];
    treasury.copy_from_slice(&data[..32]);
    let mut fee_bytes = [0u8; 8];
    fee_bytes.copy_from_slice(&data[32..40]);
    let fee_lamports = u64::from_le_bytes(fee_bytes);

    require_signer(payer)?;
    require_writable(payer)?;
    require_signer(owner)?;
    require_writable(config)?;
    require_system_program(system_program)?;

    if fee_lamports > MAX_FEE_LAMPORTS {
        return Err(LaunchpadError::FeeAboveCap.into());
    }
    if treasury == [0u8; 32] {
        return Err(LaunchpadError::InvalidTreasury.into());
    }

    let (expected, bump) = crate::state::find_pda(&[CONFIG_SEED], program_id);
    if config.address() != &expected {
        return Err(LaunchpadError::InvalidPda.into());
    }
    if config.owned_by(program_id) || config.data_len() != 0 {
        return Err(LaunchpadError::ConfigAlreadyInitialized.into());
    }

    let bump_seed = [bump];
    let seeds = [Seed::from(CONFIG_SEED), Seed::from(&bump_seed)];
    create_pda_account(payer, config, CONFIG_LEN, program_id, &seeds)?;

    let mut data = config.try_borrow_mut()?;
    Config {
        owner: *owner.address().as_array(),
        treasury,
        fee_lamports,
        bump,
    }
    .write(&mut data)?;
    Ok(())
}
