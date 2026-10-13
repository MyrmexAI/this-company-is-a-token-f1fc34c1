pub mod init;
pub mod register_launch;
pub mod set_fee;
pub mod set_treasury;

use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::instructions::{Allocate, Assign, CreateAccount, Transfer};

use crate::error::LaunchpadError;

pub fn require_signer(account: &AccountView) -> ProgramResult {
    if account.is_signer() {
        Ok(())
    } else {
        Err(ProgramError::MissingRequiredSignature)
    }
}

pub fn require_writable(account: &AccountView) -> ProgramResult {
    if account.is_writable() {
        Ok(())
    } else {
        Err(ProgramError::InvalidArgument)
    }
}

pub fn require_system_program(account: &AccountView) -> ProgramResult {
    if account.address() == &pinocchio_system::ID {
        Ok(())
    } else {
        Err(LaunchpadError::InvalidSystemProgram.into())
    }
}

/// Creates a program-owned PDA account. Also works when someone pre-funded the
/// address with lamports (a plain `CreateAccount` would fail in that case).
pub fn create_pda_account(
    payer: &AccountView,
    pda: &AccountView,
    space: usize,
    owner: &Address,
    signer_seeds: &[Seed],
) -> ProgramResult {
    let signer = Signer::from(signer_seeds);
    let rent = Rent::get()?.try_minimum_balance(space)?;

    if pda.lamports() == 0 {
        CreateAccount {
            from: payer,
            to: pda,
            lamports: rent,
            space: space as u64,
            owner,
        }
        .invoke_signed(&[signer])
    } else {
        let current = pda.lamports();
        if current < rent {
            Transfer {
                from: payer,
                to: pda,
                lamports: rent - current,
            }
            .invoke()?;
        }
        Allocate {
            account: pda,
            space: space as u64,
        }
        .invoke_signed(&[Signer::from(signer_seeds)])?;
        Assign { account: pda, owner }.invoke_signed(&[Signer::from(signer_seeds)])
    }
}
