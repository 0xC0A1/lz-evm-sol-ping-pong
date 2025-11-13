use anchor_lang::prelude::*;

use crate::errors::MyOAppError;

pub const UINT256_SIZE: usize = 32;
pub const ABA_TYPE: u16 = 2;

/// Structure representing an ABA message
pub struct AbaMessage {
    pub ball: [u8; 32],
    pub msg_type: u16,
    pub return_options: Vec<u8>,
}

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

/// Encode a uint256 value with ABA pattern (includes message type and return options).
/// This matches Solidity's `abi.encode(uint256, uint16, bytes)`.
/// 
/// # Arguments
/// * `ball` - A 32-byte array representing the uint256 in big-endian format
/// * `return_options` - Options for the return message
/// 
/// # Returns
/// * Encoded message bytes following ABI encoding: (uint256, uint16, bytes)
pub fn encode_aba(ball: &[u8; 32], return_options: &[u8]) -> Vec<u8> {
    // ABI encoding: (uint256, uint16, bytes)
    // uint256: 32 bytes (ball)
    // uint16: 32 bytes (padded, ABA_TYPE in big-endian)
    // bytes: 32 bytes offset + 32 bytes length + data
    
    let mut encoded = Vec::new();
    
    // Encode ball (uint256) - 32 bytes
    encoded.extend_from_slice(ball);
    
    // Encode msg_type (uint16) - padded to 32 bytes
    let mut msg_type_padded = [0u8; 32];
    msg_type_padded[30..32].copy_from_slice(&ABA_TYPE.to_be_bytes());
    encoded.extend_from_slice(&msg_type_padded);
    
    // Encode return_options (bytes) - offset (32 bytes) + length (32 bytes) + data
    // In ABI encoding, the offset points to where the bytes data starts
    // Offset: 32 (ball) + 32 (msg_type) + 32 (offset field) = 96
    // This matches Solidity's abi.encode(uint256, uint16, bytes)
    let offset: u64 = 96;
    // Pad offset to 32 bytes (ABI encoding requires uint256, so 32 bytes)
    let mut offset_padded = [0u8; 32];
    offset_padded[24..32].copy_from_slice(&offset.to_be_bytes());
    encoded.extend_from_slice(&offset_padded);
    
    // Length of return_options (at offset 96)
    let len: u64 = return_options.len() as u64;
    let mut len_padded = [0u8; 32];
    len_padded[24..32].copy_from_slice(&len.to_be_bytes());
    encoded.extend_from_slice(&len_padded);
    
    // Return options data (starts at offset 96 + 32 = 128)
    encoded.extend_from_slice(return_options);
    
    encoded
}

/// Decode an ABA message format.
/// Handles both vanilla (32 bytes) and ABA (>= 128 bytes) formats.
/// This matches Solidity's `abi.decode(bytes, (uint256, uint16, bytes))`.
/// 
/// # Arguments
/// * `message` - The encoded message bytes
/// 
/// # Returns
/// * `Ok(AbaMessage)` - Decoded ABA message with ball, msg_type, and return_options
/// * `Err(MyOAppError::InvalidMessageLength)` - If the message format is invalid
pub fn decode_aba(message: &[u8]) -> Result<AbaMessage> {
    // Vanilla format: 32 bytes (just uint256)
    if message.len() == UINT256_SIZE {
        let mut ball = [0u8; 32];
        ball.copy_from_slice(&message[0..32]);
        return Ok(AbaMessage {
            ball,
            msg_type: 0, // Vanilla type
            return_options: Vec::new(),
        });
    }
    
    // ABA format: minimum 128 bytes (32 uint256 + 32 uint16 padded + 32 offset + 32 length)
    // For empty return_options, the message will be exactly 128 bytes
    require!(message.len() >= 128, MyOAppError::InvalidMessageLength);
    
    // Decode ball (uint256) - first 32 bytes (bytes 0-31)
    let mut ball = [0u8; 32];
    ball.copy_from_slice(&message[0..32]);
    
    // Decode msg_type (uint16) - bytes 32-63, actual value in last 2 bytes (bytes 62-63)
    let msg_type = u16::from_be_bytes([message[62], message[63]]);
    
    // Decode return_options offset - bytes 64-95, actual value in last 8 bytes (bytes 88-95)
    // The offset is a uint256 (32 bytes), but we only need the last 8 bytes for the u64 value
    let offset = u64::from_be_bytes([
        message[88], message[89], message[90], message[91],
        message[92], message[93], message[94], message[95],
    ]) as usize;
    
    // Validate offset is reasonable (should point to where the length field starts)
    // In ABI encoding for (uint256, uint16, bytes), the offset is 96
    // Offset: 32 (ball) + 32 (msg_type) + 32 (offset field) = 96
    require!(offset >= 96, MyOAppError::InvalidMessageLength);
    require!(message.len() >= offset + 32, MyOAppError::InvalidMessageLength);
    
    // Decode return_options length - bytes at offset, actual value in last 8 bytes
    // The length is a uint256 (32 bytes), but we only need the last 8 bytes for the u64 value
    let len = u64::from_be_bytes([
        message[offset + 24], message[offset + 25], message[offset + 26], message[offset + 27],
        message[offset + 28], message[offset + 29], message[offset + 30], message[offset + 31],
    ]) as usize;
    
    // Validate we have enough bytes for the length field and the data
    require!(message.len() >= offset + 32 + len, MyOAppError::InvalidMessageLength);
    
    // Decode return_options data - starts after the length field (offset + 32)
    let return_options = if len > 0 {
        message[offset + 32..offset + 32 + len].to_vec()
    } else {
        Vec::new()
    };
    
    Ok(AbaMessage {
        ball,
        msg_type,
        return_options,
    })
}
