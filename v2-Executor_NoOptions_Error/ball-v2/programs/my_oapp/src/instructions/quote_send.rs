use crate::{consts::*, *};
use anchor_lang::prelude::*;
use ethnum::U256;
use oapp::endpoint::{
    instructions::QuoteParams, state::EndpointSettings, ENDPOINT_SEED, ID as ENDPOINT_ID,
};

#[derive(Accounts)]
#[instruction(params: QuoteSendParams)]
pub struct QuoteSend<'info> {
    #[account(seeds = [STORE_SEED], bump = store.bump)]
    pub store: Account<'info, Store>,
    #[account(
    seeds = [
        PEER_SEED,
        store.key().as_ref(),
        &params.dst_eid.to_be_bytes()
    ],
    bump = peer.bump
    )]
    pub peer: Account<'info, PeerConfig>,
    #[account(seeds = [ENDPOINT_SEED], bump = endpoint.bump, seeds::program = ENDPOINT_ID)]
    pub endpoint: Account<'info, EndpointSettings>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct QuoteSendParams {
    pub dst_eid: u32,
    pub receiver: [u8; 32],
    pub return_options: Vec<u8>, // Options for the return message (B→A)
    pub options: Vec<u8>, // Additional options for the initial send (A→B)
    pub pay_in_lz_token: bool,
}

impl<'info> QuoteSend<'info> {
    pub fn apply(ctx: &Context<QuoteSend>, params: &QuoteSendParams) -> Result<MessagingFee> {
        // Encode ABA message for quoting
        let ball = ctx.accounts.store.ball;
        let ball_ethnum = U256::from_be_bytes(ball);
        let new_ball = ball_ethnum.saturating_sub(U256::ONE).to_be_bytes();
        
        // Encode ABA message with return options
        let message = uint256_msg_codec::encode_aba(&new_ball, &params.return_options);

        // Ask the Endpoint how much a send would cost
        let quote_params = QuoteParams {
            sender: ctx.accounts.store.key(),
            dst_eid: params.dst_eid,
            receiver: params.receiver,
            message,
            pay_in_lz_token: params.pay_in_lz_token,
            options: ctx
                .accounts
                .peer
                .enforced_options
                .combine_options(&None::<Vec<u8>>, &params.options)?,
        };
        oapp::endpoint_cpi::quote(ENDPOINT_ID, ctx.remaining_accounts, quote_params)
    }
}
