use {
    num_derive::FromPrimitive,
    solana_program::{
        decode_error::DecodeError,
        msg,
        program_error::{PrintProgramError, ProgramError},
    },
    thiserror::Error,
};

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum TokenPoolError {
    /// Invalid Data
    #[error("InvalidData")]
    InvalidData,
    /// Minimum amount is greater than Target amount
    #[error("WrongAmountData")]
    WrongAmountData,
    /// Maximum members in token pool should be two
    #[error("MaxMemberAtleastTwo")]
    MaxMemberAtleastTwo,
    /// Target amount for the pool has been collected
    #[error("TargetBalanceReached")]
    TargetBalanceReached,
    /// Member already exists in pool
    #[error("MemberAlreadyExists")]
    MemberAlreadyExists,
    /// Token pool has been filled
    #[error("NoMemberSpaceLeft")]
    NoMemberSpaceLeft,
    /// LastMember has to reach target amount
    #[error("InsufficientFundsAsLastMember")]
    InsufficientFundsAsLastMember,
    /// Token pool is not initialized
    #[error("UninitializedTokenPool")]
    UninitializedTokenPool,
    /// Member is not part of token pool
    #[error("MemberNotInPool")]
    MemberNotInPool,
    /// Escrow stage is invalid
    #[error("InvalidEscrowStage")]
    InvalidEscrowStage,
}

impl From<TokenPoolError> for ProgramError {
    fn from(e: TokenPoolError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for TokenPoolError {
    fn type_of() -> &'static str {
        "Token Pool Error"
    }
}

impl PrintProgramError for TokenPoolError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + num_traits::FromPrimitive,
    {
        match self {
            TokenPoolError::InvalidData => msg!("Invalid Data"),
            TokenPoolError::WrongAmountData => msg!("Minimum amount is greater than Target amount"),
            TokenPoolError::MaxMemberAtleastTwo => {
                msg!("Maximum members in token pool should be two")
            }
            TokenPoolError::TargetBalanceReached => {
                msg!("Target amount for the pool has been collected")
            }
            TokenPoolError::MemberAlreadyExists => msg!("Member already exists in pool"),
            TokenPoolError::NoMemberSpaceLeft => msg!("Token pool has been filled"),
            TokenPoolError::InsufficientFundsAsLastMember => {
                msg!("LastMember has to reach target amount")
            }
            TokenPoolError::UninitializedTokenPool => msg!("Token pool is not initialized"),
            TokenPoolError::MemberNotInPool => msg!("Member is not part of token pool"),
            TokenPoolError::InvalidEscrowStage => msg!("Escrow stage is invalid"),
        }
    }
}
