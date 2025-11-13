pub const LZ_RECEIVE_TYPES_SEED: &[u8] = b"LzReceiveTypes"; // The Executor relies on this exact seed to derive the LzReceiveTypes PDA. Keep it the same.
pub const STORE_SEED: &[u8] = b"Store"; // You are free to edit this seed.
pub const PEER_SEED: &[u8] = b"Peer"; // Not used by the Executor.

// Base estimate for Solana -> Ethereum messaging fee (in lamports)
// This is used as a reference point for estimating return message fees in ABA pattern
// Actual cost may vary, so we use a multiplier for safety
pub const BASE_SOL_TO_ETH_FEE: u64 = 6_365_917; // Base cost for Sol->ETH trip
pub const RETURN_FEE_MULTIPLIER: u64 = 2; // Use 2x as safety buffer for return message
