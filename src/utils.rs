use num_enum::{IntoPrimitive, TryFromPrimitive};
use solana_program::program_error::ProgramError;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum AccountDiscriminator {
    ManagedProof = 100,
    DelegatedStake = 101,
    DelegatedBoost = 102,
}

pub trait Discriminator {
    fn discriminator() -> AccountDiscriminator;
}

pub trait AccountDeserialize {
    fn try_from_bytes(data: &[u8]) -> Result<&Self, ProgramError>;
    fn try_from_bytes_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError>;
}

#[macro_export]
macro_rules! impl_to_bytes {
    ($struct_name:ident) => {
        impl $struct_name {
            pub fn to_bytes(&self) -> &[u8] {
                bytemuck::bytes_of(self)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_account_from_bytes {
    ($struct_name:ident) => {
        impl crate::utils::AccountDeserialize for $struct_name {
            fn try_from_bytes(
                data: &[u8],
            ) -> Result<&Self, solana_program::program_error::ProgramError> {
                if (Self::discriminator() as u8).ne(&data[0]) {
                    return Err(solana_program::program_error::ProgramError::InvalidAccountData);
                }
                bytemuck::try_from_bytes::<Self>(&data[8..]).or(Err(
                    solana_program::program_error::ProgramError::InvalidAccountData,
                ))
            }
            fn try_from_bytes_mut(
                data: &mut [u8],
            ) -> Result<&mut Self, solana_program::program_error::ProgramError> {
                if (Self::discriminator() as u8).ne(&data[0]) {
                    return Err(solana_program::program_error::ProgramError::InvalidAccountData);
                }
                bytemuck::try_from_bytes_mut::<Self>(&mut data[8..]).or(Err(
                    solana_program::program_error::ProgramError::InvalidAccountData,
                ))
            }
        }
    };
}

// Load and verify the account is initialized and has the correct program id
#[macro_export]
macro_rules! impl_account_from_account_info {
    ($struct_name:ident) => {
        impl $struct_name {
            pub fn from_account_info(account_info: &AccountInfo) -> Result<Self, ProgramError> {
                if *account_info.owner != crate::id() {
                    return Err(ProgramError::IncorrectProgramId);
                }

                if account_info.data_is_empty() {
                    return Err(ProgramError::UninitializedAccount);
                }

                if let Ok(account) = account_info.data.try_borrow() {
                    let parsed_data = $struct_name::try_from_bytes(&account)?;
                    return Ok(parsed_data.clone());
                } else {
                    return Err(ProgramError::AccountBorrowFailed);
                };
            }
        }
    };
}

#[macro_export]
macro_rules! impl_instruction_from_bytes {
    ($struct_name:ident) => {
        impl $struct_name {
            pub fn try_from_bytes(
                data: &[u8],
            ) -> Result<&Self, solana_program::program_error::ProgramError> {
                bytemuck::try_from_bytes::<Self>(data).or(Err(
                    solana_program::program_error::ProgramError::InvalidInstructionData,
                ))
            }
        }
    };
}
