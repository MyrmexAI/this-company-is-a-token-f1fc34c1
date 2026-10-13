use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

use super::{require_signer, require_writable};
use crate::{
    error::LaunchpadError,
    state::{load_config, Config},
};

/// Accounts: 0 owner (signer) | 1 config PDA (writable)
/// Data: treasury (32)
pub fn process(program_id: &Address, accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [owner, config, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let treasury: [u8; 32] = data
        .try_into()
        .map_err(|_| LaunchpadError::InvalidInstructionData)?;

    require_signer(owner)?;
    require_writable(config)?;
    let cfg = load_config(program_id, config)?;
    if owner.address().as_array() != &cfg.owner {
        return Err(LaunchpadError::NotOwner.into());
    }
    if treasury == [0u8; 32] {
        return Err(LaunchpadError::InvalidTreasury.into());
    }

    let mut config_data = config.try_borrow_mut()?;
    Config::set_treasury(&mut config_data, &treasury);
    Ok(())
}
