use pinocchio::error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchpadError {
    InvalidProgramId = 0,
    InvalidInstructionData = 1,
    FeeAboveCap = 2,
    NotOwner = 3,
    InvalidConfig = 4,
    ConfigAlreadyInitialized = 5,
    InvalidTreasury = 6,
    InvalidPda = 7,
    AlreadyRegistered = 8,
    InvalidMint = 9,
    InvalidSystemProgram = 10,
}

impl From<LaunchpadError> for ProgramError {
    fn from(e: LaunchpadError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
