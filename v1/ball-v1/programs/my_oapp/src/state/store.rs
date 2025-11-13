use crate::*;
use ethnum::U256;

#[account]
pub struct Store {
    // Store admin (Signer).
    pub admin: Pubkey,
    // Store account bump for Pda derivation.
    pub bump: u8,
    // Endpoint program ID.
    pub endpoint_program: Pubkey,
    // Current ball value.
    pub ball: [u8; 32],
}

impl Store {
    pub const SIZE: usize = 8 + std::mem::size_of::<Self>();

    /// Initial ball value matching Ethereum contract: 100000000000000000000 (100 * 10^18)
    pub const INITIAL_BALL: u128 = 100_000_000_000_000_000_000u128;

    pub fn new(admin: Pubkey, bump: u8, endpoint_program: Pubkey) -> Self {
        // Initialize ball with the same value as Ethereum contract
        let initial_ball = U256::from(Self::INITIAL_BALL);
        Self { admin, bump, endpoint_program, ball: initial_ball.to_be_bytes() }
    }

    pub fn set_ball(&mut self, ball: [u8; 32]) {
        self.ball = ball;
    }
}

// The LzReceiveTypesAccounts PDA is used by the Executor as a prerequisite to calling `lz_receive`.
#[account]
pub struct LzReceiveTypesAccounts {
    pub store: Pubkey, // This is required and should be consistent.
}

impl LzReceiveTypesAccounts {
    pub const SIZE: usize = 8 + std::mem::size_of::<Self>();

    pub fn new(store: Pubkey) -> Self {
        Self { store }
    }
}
