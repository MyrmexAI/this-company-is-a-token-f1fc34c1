use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

use super::{require_signer, require_writable};
use crate::{
    error::LaunchpadError,
    state::{load_config, Config, MAX_FEE_LAMPORTS},
};

/// Accounts: 0 owner (signer) | 1 config PDA (writable)
/// Data: fee_lamports (8, LE)
pub fn process(program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [owner, config, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let fee_bytes: [u8; 8] = data
        .try_into()
        .map_err(|_| LaunchpadError::InvalidInstructionData)?;
    let fee_lamports = u64::from_le_bytes(fee_bytes);

    require_signer(owner)?;
    require_writable(config)?;
    let cfg = load_config(program_id, config)?;
    if owner.address().as_array() != &cfg.owner {
        return Err(LaunchpadError::NotOwner.into());
    }
    if fee_lamports > MAX_FEE_LAMPORTS {
        return Err(LaunchpadError::FeeAboveCap.into());
    }

    let mut config_data = config.try_borrow_mut()?;
    Config::set_fee(&mut config_data, fee_lamports);
    Ok(())
}
