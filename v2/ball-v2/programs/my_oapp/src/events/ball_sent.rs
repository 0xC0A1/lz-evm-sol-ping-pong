use anchor_lang::prelude::*;

#[event]
pub struct BallSent {
    pub current_ball: Vec<u8>,
    pub new_ball: Vec<u8>,
    pub current_ball_str: String,
    pub new_ball_str: String,
    pub dst_eid: u32,
}
