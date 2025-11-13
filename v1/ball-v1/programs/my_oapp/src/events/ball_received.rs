use anchor_lang::prelude::*;

#[event]
pub struct BallReceived {
    pub old_ball: Vec<u8>,
    pub new_ball: Vec<u8>,
    pub old_ball_str: String,
    pub new_ball_str: String,
    pub src_eid: u32,
}
