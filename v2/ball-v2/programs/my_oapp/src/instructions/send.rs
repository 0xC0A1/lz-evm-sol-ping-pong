use crate::{consts::*, *};
use anchor_lang::prelude::*;
use ethnum::U256;
use oapp::endpoint::{
    instructions::SendParams, state::EndpointSettings, ENDPOINT_SEED, ID as ENDPOINT_ID,
};

#[derive(Accounts)]
#[instruction(params: SendMessageParams)]
pub struct Send<'info> {
    #[account(
        seeds = [
            PEER_SEED,
            &store.key().to_bytes(),
            &params.dst_eid.to_be_bytes()
        ],
        bump = peer.bump
    )]
    /// Configuration for the destination chain. Holds the peer address and any
    /// enforced messaging options.
    pub peer: Account<'info, PeerConfig>,
    #[account(seeds = [STORE_SEED], bump = store.bump)]
    /// OApp Store PDA that signs the send instruction
    pub store: Account<'info, Store>,
    #[account(seeds = [ENDPOINT_SEED], bump = endpoint.bump, seeds::program = ENDPOINT_ID)]
    pub endpoint: Account<'info, EndpointSettings>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct SendMessageParams {
    pub dst_eid: u32,
    pub return_options: Vec<u8>, // Options for the return message (B→A)
    pub options: Vec<u8>, // Additional options for the initial send (A→B)
    pub native_fee: u64,
    pub lz_token_fee: u64,
}

impl<'info> Send<'info> {
    pub fn apply(ctx: &mut Context<Send>, params: &SendMessageParams) -> Result<()> {
        // Prepare the seeds for the OApp Store PDA, which is used to sign the CPI call to the Endpoint program.
        let seeds: &[&[u8]] = &[STORE_SEED, &[ctx.accounts.store.bump]];

        let ball = ctx.accounts.store.ball;
        let ball_ethnum = U256::from_be_bytes(ball);
        let new_ball_ethnum = ball_ethnum.saturating_sub(U256::ONE);
        let new_ball = new_ball_ethnum.to_be_bytes();
        
        // Encode ABA message with return options
        let message = uint256_msg_codec::encode_aba(&new_ball, &params.return_options);

        // Emit event tracking the ball value
        emit!(crate::events::BallSent {
            current_ball: ball.to_vec(),
            new_ball: new_ball.to_vec(),
            current_ball_str: ball_ethnum.to_string(),
            new_ball_str: new_ball_ethnum.to_string(),
            dst_eid: params.dst_eid,
        });

        // Prepare the SendParams for the Endpoint::send CPI call.
        // For ABA pattern, options should include ExecutorLzReceiveOption with return gas
        // The options are typically built off-chain using the SDK, but we combine with enforced options here
        let send_params = SendParams {
            dst_eid: params.dst_eid,
            receiver: ctx.accounts.peer.peer_address,
            message,
            options: ctx
                .accounts
                .peer
                .enforced_options
                .combine_options(&None::<Vec<u8>>, &params.options)?,
            native_fee: params.native_fee,
            lz_token_fee: params.lz_token_fee,
        };
        // Call the Endpoint::send CPI to send the message.
        oapp::endpoint_cpi::send(
            ENDPOINT_ID,
            ctx.accounts.store.key(),
            ctx.remaining_accounts,
            seeds,
            send_params,
        )?;
        Ok(())
    }
}
