
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
    pubkey::Pubkey,
};
// impl Sealed for Request {}

// impl Pack for Request {
//     const LEN: usize = 1 + 32 + 32 + 32 + 32 + 32 + 32 + 32 + 32 + 8 + 8 + 8;

//     fn pack_into_slice(&self, dst: &mut [u8]) {
//         let mut slice = dst;
//         self.serialize(&mut slice).unwrap()
//     }

//     fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
//         let mut p = src;
//         Request::deserialize(&mut p).map_err(|_| {
//             msg!("Failed to deserialize name record");
//             ProgramError::InvalidAccountData
//         })
//     }
// }
