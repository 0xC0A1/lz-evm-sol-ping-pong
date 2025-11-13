use anchor_lang::prelude::error_code;

#[error_code]
pub enum MyOAppError {
    InvalidBallLength,
    InvalidMessageLength,
    InvalidMessageType, // Message is not ABA type
}
