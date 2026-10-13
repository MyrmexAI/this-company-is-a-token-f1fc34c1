#![no_std]

//! AntPad launchpad registry.
//!
//! Tokens are created and traded on pump.fun. This program only records launches
//! made through AntPad and charges a fixed platform fee in SOL, paid by the creator
//! straight to the owner's treasury wallet. It never holds user funds.

pub mod error;
pub mod instructions;
pub mod state;

use pinocchio::{address::declare_id, AccountView, Address, ProgramResult};

use error::LaunchpadError;

// Placeholder: the platform replaces this with the real program id before deploying.
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint {
    use super::*;

    pinocchio::program_entrypoint!(process_instruction);
    pinocchio::no_allocator!();
    pinocchio::nostd_panic_handler!();
}

pub const IX_INIT: u8 = 0;
pub const IX_SET_FEE: u8 = 1;
pub const IX_SET_TREASURY: u8 = 2;
pub const IX_REGISTER_LAUNCH: u8 = 3;

pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    if program_id != &ID {
        return Err(LaunchpadError::InvalidProgramId.into());
    }
    let (disc, rest) = data
        .split_first()
        .ok_or(LaunchpadError::InvalidInstructionData)?;
    match *disc {
        IX_INIT => instructions::init::process(program_id, accounts, rest),
        IX_SET_FEE => instructions::set_fee::process(program_id, accounts, rest),
        IX_SET_TREASURY => instructions::set_treasury::process(program_id, accounts, rest),
        IX_REGISTER_LAUNCH => instructions::register_launch::process(program_id, accounts, rest),
        _ => Err(LaunchpadError::InvalidInstructionData.into()),
    }
}
