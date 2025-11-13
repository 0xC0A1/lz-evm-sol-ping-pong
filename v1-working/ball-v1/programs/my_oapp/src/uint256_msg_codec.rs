use anchor_lang::prelude::*;

use crate::errors::MyOAppError;

pub const UINT256_SIZE: usize = 32;

/// Encode a uint256 value (represented as 32 bytes in big-endian) into a message format.
/// This matches Solidity's `abi.encode(uint256)` which produces 32 bytes in big-endian format.
/// 
/// # Arguments
/// * `value` - A 32-byte array representing the uint256 in big-endian format
/// 
/// # Example
/// ```
/// use ethnum::U256;
/// let value = U256::from(100_000_000_000_000_000_000u128);
/// let bytes = value.to_be_bytes();
/// let encoded = encode(&bytes);
/// ```
pub fn encode(value: &[u8; 32]) -> Vec<u8> {
    value.to_vec()
}

/// Decode a message into a uint256 value (32 bytes in big-endian format).
/// Returns an error if the message is not exactly 32 bytes.
/// This matches Solidity's `abi.decode(bytes, (uint256))`.
/// 
/// # Arguments
/// * `message` - The encoded message bytes (must be exactly 32 bytes)
/// 
/// # Returns
/// * `Ok([u8; 32])` - The uint256 value as a 32-byte array in big-endian format
/// * `Err(MsgCodecError::InvalidMsgLength)` - If the message is not exactly 32 bytes
/// 
/// # Example
/// ```
/// use ethnum::U256;
/// let bytes = decode(&message)?;
/// let value = U256::from_be_bytes(bytes);  // Convert to U256 using big-endian
/// ```
pub fn decode(message: &[u8]) -> Result<[u8; 32]> {
    require!(message.len() == UINT256_SIZE, MyOAppError::InvalidMessageLength);
    let mut result = [0u8; 32];
    result.copy_from_slice(message);
    Ok(result)
}
