use crate::{consts::*, errors::MyOAppError, *};
use anchor_lang::prelude::*;
use ethnum::U256;
use oapp::{
    endpoint::{
        cpi::accounts::Clear,
        instructions::{ClearParams, SendParams},
        ConstructCPIContext,
        ID as ENDPOINT_ID,
    },
    LzReceiveParams,
};

#[derive(Accounts)]
#[instruction(params: LzReceiveParams)]
pub struct LzReceive<'info> {
    /// OApp Store PDA.  This account represents the "address" of your OApp on
    /// Solana and can contain any state relevant to your application.
    /// Customize the fields in `Store` as needed.
    #[account(mut, seeds = [STORE_SEED], bump = store.bump)]
    pub store: Account<'info, Store>,
    /// Peer config PDA for the sending chain. Ensures `params.sender` can only be the allowed peer from that remote chain.
    #[account(
        seeds = [PEER_SEED, &store.key().to_bytes(), &params.src_eid.to_be_bytes()],
        bump = peer.bump,
        constraint = params.sender == peer.peer_address
    )]
    pub peer: Account<'info, PeerConfig>,
}

impl LzReceive<'_> {
    pub fn apply(ctx: &mut Context<LzReceive>, params: &LzReceiveParams) -> Result<()> {
        // The OApp Store PDA is used to sign the CPI to the Endpoint program.
        let seeds: &[&[u8]] = &[STORE_SEED, &[ctx.accounts.store.bump]];

        // The first Clear::MIN_ACCOUNTS_LEN accounts were returned by
        // `lz_receive_types` and are required for Endpoint::clear
        let accounts_for_clear = &ctx.remaining_accounts[0..Clear::MIN_ACCOUNTS_LEN];
        // Call the Endpoint::clear CPI to clear the message from the Endpoint program.
        // This is necessary to ensure the message is processed only once and to
        // prevent replays.
        let _ = oapp::endpoint_cpi::clear(
            ENDPOINT_ID,
            ctx.accounts.store.key(),
            accounts_for_clear,
            seeds,
            ClearParams {
                receiver: ctx.accounts.store.key(),
                src_eid: params.src_eid,
                sender: params.sender,
                nonce: params.nonce,
                guid: params.guid,
                message: params.message.clone(),
            },
        )?;

        // Decode ABA message - only ABA flows are supported
        let aba_msg = uint256_msg_codec::decode_aba(&params.message)?;
        
        // Verify this is an ABA message type
        require!(
            aba_msg.msg_type == uint256_msg_codec::ABA_TYPE,
            MyOAppError::InvalidMessageType
        );

        // Update ball
        let store = &mut ctx.accounts.store;
        let old_ball = store.ball;
        let old_ball_ethnum = U256::from_be_bytes(old_ball);
        let new_ball_ethnum = U256::from_be_bytes(aba_msg.ball);
        store.set_ball(aba_msg.ball);

        // Emit event tracking the ball value
        emit!(crate::events::BallReceived {
            old_ball: old_ball.to_vec(),
            new_ball: aba_msg.ball.to_vec(),
            old_ball_str: old_ball_ethnum.to_string(),
            new_ball_str: new_ball_ethnum.to_string(),
            src_eid: params.src_eid,
        });

        // ABA pattern: always send response back
        // Decrement ball for return message
        let ball_ethnum = U256::from_be_bytes(aba_msg.ball);
        let return_ball_ethnum = ball_ethnum.saturating_sub(U256::ONE);
        let return_ball = return_ball_ethnum.to_be_bytes();
        
        // Encode return message (vanilla type - return messages are always vanilla)
        let return_message = uint256_msg_codec::encode(&return_ball);
        
        // Update store with decremented ball
        store.set_ball(return_ball);
        
        // Prepare options for return message
        // Use the return_options from the ABA message (same as Ethereum does)
        // The enforced_options will combine with them to ensure proper formatting
        // Note: Even if return_options are empty, enforced_options should add ExecutorLzReceiveOption
        // for vanilla messages (msgType 1) to ensure the Executor can execute the return message
        // The enforced_options are configured via layerzero.config.ts and should include
        // ExecutorLzReceiveOption with appropriate gas for the return message execution
        let return_options = ctx
            .accounts
            .peer
            .enforced_options
            .combine_options(&None::<Vec<u8>>, &aba_msg.return_options)?;
        
        // Prepare SendParams for the return message
        // Send back to src_eid (the origin chain)
        // Estimate return message fee: Use 2x the base Sol->ETH fee as a safety buffer
        // This accounts for:
        // - Base messaging cost (Sol->ETH)
        // - Network conditions and gas price variations
        // - Safety margin for successful execution
        // Note: The actual fee may vary, but this provides a reasonable estimate.
        // The executor should ensure sufficient native fee is forwarded in the initial message.
        let estimated_return_fee = consts::BASE_SOL_TO_ETH_FEE
            .checked_mul(consts::RETURN_FEE_MULTIPLIER)
            .unwrap_or(consts::BASE_SOL_TO_ETH_FEE * consts::RETURN_FEE_MULTIPLIER);
        
        let send_params = SendParams {
            dst_eid: params.src_eid,
            receiver: ctx.accounts.peer.peer_address,
            message: return_message,
            options: return_options,
            native_fee: estimated_return_fee,
            lz_token_fee: 0, // No LZ token fee for return
        };
        
        // Send return message via Endpoint CPI
        // Note: remaining_accounts after Clear::MIN_ACCOUNTS_LEN should contain
        // accounts needed for Send CPI (returned by send_types instruction)
        // These accounts are typically fetched off-chain using the endpoint SDK's
        // getSendIXAccountMetaForCPI method
        
        // For ABA pattern, the return message accounts should be provided
        // as additional remaining_accounts after the clear accounts
        let accounts_for_send = &ctx.remaining_accounts[Clear::MIN_ACCOUNTS_LEN..];
        
        oapp::endpoint_cpi::send(
            ENDPOINT_ID,
            ctx.accounts.store.key(),
            accounts_for_send,
            seeds,
            send_params,
        )?;

        Ok(())
    }
}
