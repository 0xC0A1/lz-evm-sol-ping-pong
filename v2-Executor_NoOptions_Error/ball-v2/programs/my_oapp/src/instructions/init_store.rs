use crate::{consts::*, *};

use oapp::endpoint::{instructions::RegisterOAppParams, ID as ENDPOINT_ID};

#[derive(Accounts)]
#[instruction(params: InitStoreParams)]
pub struct InitStore<'info> {
    #[account(
        mut,
        // Restrict address to me (Deployer).
        address = pubkey!("8EJpvGttUbvSr99iPvT3w2H1NtUGZkmqvThJkPLKfNiM")
    )]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = Store::SIZE,
        seeds = [STORE_SEED], // You can namespace this further if your program manages multiple stores.
        // e.g. If there can be a store for each user, you can use something like:
        // seeds = [STORE_SEED, &user.key().as_ref()]
        bump
    )]
    pub store: Account<'info, Store>,
    #[account(
        init,
        payer = payer,
        space = LzReceiveTypesAccounts::SIZE,
        seeds = [LZ_RECEIVE_TYPES_SEED, &store.key().to_bytes()],
        bump
    )]
    pub lz_receive_types_accounts: Account<'info, LzReceiveTypesAccounts>,
    pub system_program: Program<'info, System>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InitStoreParams {
    pub admin: Pubkey,
    pub endpoint: Pubkey,
}

impl InitStore<'_> {
    pub fn apply(ctx: &mut Context<InitStore>, params: &InitStoreParams) -> Result<()> {
        ctx.accounts
            .store
            .set_inner(Store::new(params.admin, ctx.bumps.store, params.endpoint));
        ctx.accounts
            .lz_receive_types_accounts
            .set_inner(LzReceiveTypesAccounts::new(ctx.accounts.store.key()));
        // the above lines are required for all OApp implementations

        // Prepare the delegate address for the OApp registration.
        let register_params = RegisterOAppParams { delegate: ctx.accounts.store.admin };

        // The Store PDA 'signs' CPI to the Endpoint program to register the OApp.
        let seeds: &[&[u8]] = &[STORE_SEED, &[ctx.accounts.store.bump]];
        oapp::endpoint_cpi::register_oapp(
            ENDPOINT_ID,
            ctx.accounts.store.key(),
            ctx.remaining_accounts,
            seeds,
            register_params,
        )?;

        Ok(())
    }
}
