# Ping Pong Eth<->Solana with LayerZero

Participating in LayerZero challenge.

## How to run & deploy:

First, set up Solana dev tools.

Then, create a proper `.env` file following the layout of `.env.example`.

Modify `v1/ball-v1/programs/my_oapp/src/instructions/init_store.rs` and change `address = pubkey!("8EJpvGttUbvSr99iPvT3w2H1NtUGZkmqvThJkPLKfNiM")` with an address under your control.

```sh
cd v1/ball-v1

# Generate a new Solana keypair for the program
solana-keygen new -o target/deploy/my_oapp-keypair.json
# Set anchor keys
anchor keys sync
# Get created pubkey
anchor keys list
# Build program
anchor build -v -e MYOAPP_ID=<OAPP_PROGRAM_ID>

# You'd also have to manually modify:
# v1/ball-v1/lib/scripts/generate.ts with the new program address.

# Deploy program to devnet
solana program deploy --program-id target/deploy/my_oapp-keypair.json target/verifiable/my_oapp.so -u devnet
# Init OApp Store
npx hardhat lz:oapp:solana:create --eid 40168 --program-id <PROGRAM_ID>
# Deploy Eth OApp
npx hardhat lz:deploy
# Select ethereum-sepolia

# Wiring

# Run init-config
npx hardhat lz:oapp:solana:init-config --oapp-config layerzero.config.ts
# Run wire task
npx hardhat lz:oapp:wire --oapp-config layerzero.config.ts

# Now, test the communications live!

# Sepolia -> Solana
npx hardhat \
--network arbitrum-sepolia lz:oapp:send \
--from-eid 40168 --dst-eid 40168

# Solana -> Sepolia
npx hardhat \
lz:oapp:send --from-eid 40168 \
--dst-eid 40161

```

## LayerZero messaging

LayerZero's tooling starts with three main endpoints:

- `quote` quotes the amount in native gas or LZ token to pay for the transaction.
- `send` leverages a quote from the `quote` endpoint to pay for the gas on the sender's and receiver's end.
- `_lzReceive` is an override to define custom contract logic for when a message is received from another endpoint.

On Solana, there are a couple of extra primitives that are needed to get an LZ OApp working:

- `init_store` This one is not 100% necessary or custom to LayerZero; it's a PDA that gets associated to the `lz_receive_types` instruction.
- `lz_receive_types` is a definition for what accounts to pass at runtime into the `lz_receive` call.
- `set_peer_config` similar to a wiring task to set up remote chains (or peers) to store them.
- `quote_send` quotes the gas price to go from Solana to Ethereum.
- `send` sends the message to the remote chain (peer).
- `lz_receive` instruction that is fired by an executor on the Solana chain.

## What went well, what didn't

### What went well

✅ LayerZero's documentation is very detailed and straightforward; their AI tool enabled me to find key enablers for this.  
✅ LayerZero provided a good starting point template for EVM+SOL, using tools which I'm very familiar with (Solana Umi, CodeGen, HardHat).  
✅ Initially, I wanted to enforce a nonce to avoid "ball dropping and desyncs" and have the ball at all times, but then I realized `lz_receive` took care of that for me.  
✅ I was able to easily modify the HardHat tasks for sending the ball.  
✅ Used U256 in both Eth and Solana to avoid numerical issues.

### What I wish was better

⚠️ LayerZero's template had some bugs that I had to fix using my prior knowledge of the tooling: - TypeScript compile errors on the `gen:api` command that generates the Umi SDK using `kinobi` (now Codama).

### What didn't go well

😞 LayerZero's documentation around A->B->A model was found, but I had a lot of issues sending data through the wire (see `v2`).  
😞 I wanted to do an infinite A-B-A-B... loop by leveraging Ethereum smart wallets and PDAs on Solana but figured out
it might not be possible using the current SDK on Solana.  
😞 Getting quotes for A-B-A-B... in itself can be really hard, and end up wasting gas.

## What I'd improve next time

- Figure out a way to implement an A->B->A->B finite loop with a static gas allocation or:
    - Query the remote chain via the `lzRead` primitive for proper gas allocation.
- Add more instructions for reset and ball management; also introduce a concept of "ball drop penalties" to make the ping-pong more fun and competitive across chain stability.
- Create a proper `Serve -> Ping -> Pong -> Drop? -> Serve ...` flow with a proper scoring mechanism.

## Chain differences

Data encoding could be a bit tricky and cumbersome since `abi` encoding can be vastly different from `borsh`.
Solana's composability makes it a bit harder to get in at the beginning for any Ethereum developer but it pays off eventually.
Solana's need for account passing introduces a couple of challenges when designing more customized flows of data, like DeFi, for example.

## Key takeaways

Using LayerZero is easy enough with their templates, but I can't imagine setting up a project directly from zero just by following their docs;
seems like there's a lot of things like setting up the wiring that isn't properly documented for the raw SDKs and the best way to understand the
SDK is by getting a hands-on approach for it.

A sort of a sad time for me was when I faced a wall when I was starting to pass the send to EVM from Solana, got an `Executor_NoOptions` error
which was also hard to debug.

Finality is mostly the price that you pay with a product like LayerZero, where transactions take up to five minutes to land. Can you imagine
a ping pong ball taking that amount of time to reach the other side of the table? Hahaha.

Alright, that's it from me,

Kev.