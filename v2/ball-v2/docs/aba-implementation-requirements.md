# ABA (Ping-Pong) Pattern Implementation Requirements

## Overview

The ABA pattern enables nested messaging where a message sent from Chain A to Chain B triggers another message back to Chain A (`A` → `B` → `A`). This document outlines all the changes needed to implement this pattern for both the EVM (Solidity) and Solana contracts.

## Quick Summary

### What Needs to Change

1. **Message Format**: Extend encoding to include message type and return options
2. **Send Function**: Add `sendABA()` that calculates total gas for both legs
3. **Receive Function**: Modify `_lzReceive()` to detect ABA messages and send responses
4. **Gas Handling**: Forward sufficient gas to cover return message execution

### Key Challenges

- **Gas Estimation**: Cannot directly quote B→A cost from Chain A
  - Solution: User provides estimate, use multiplier, or query via lzRead
- **Gas Forwarding**: Must forward return gas through the initial message
  - Solution: Include return gas in `ExecutorLzReceiveOption` 
- **Message Type Detection**: Need to distinguish ABA from vanilla messages
  - Solution: Include message type in payload

### Implementation Complexity

- **EVM**: Medium complexity - straightforward gas forwarding
- **Solana**: Higher complexity - requires peer config lookup and CPI handling

## Key Requirements

### 1. Message Encoding Changes

Both contracts need to encode additional information in the message payload:

**Current encoding:**
- Just the `ball` value (32 bytes)

**ABA encoding needed:**
- `ball` value (32 bytes)
- `msgType` (uint16, 2 bytes) - to distinguish between:
  - `VANILLA_TYPE = 1` - regular one-way message
  - `ABA_TYPE = 2` - ABA pattern (should trigger response)
- `returnOptions` (bytes) - execution options for the B→A return message

**Total message size:** Variable (32 + 2 + variable length options)

### 2. Gas Calculation & Forwarding

**Critical requirement:** The source OApp (A) must pay for BOTH legs of the journey:
- A→B: Initial message send
- B→A: Return message response

This requires:
1. **Pre-calculating B→A gas cost** before sending A→B
2. **Including return gas in A→B options** via `ExecutorLzReceiveOption`
3. **Forwarding sufficient `msg.value`** to cover both transactions

### 3. Return Message Handling

When Chain B receives an ABA-type message:
- Process the message normally
- Extract `returnOptions` from the message
- Send a response back to Chain A using `_origin.srcEid`
- Use the forwarded gas to pay for the return message

---

## EVM Contract (MyOApp.sol) Changes

### 1. Update Message Codec

**File: `contracts/libs/Uint256MsgCodec.sol`**

Add ABA message type and encoding/decoding functions:

```solidity
library Uint256MsgCodec {
    uint8 public constant VANILLA_TYPE = 1;
    uint8 public constant ABA_TYPE = 2;  // NEW: ABA pattern type

    // Existing encode/decode for vanilla messages...
    
    /// @notice Encodes a uint256 with message type and return options for ABA pattern
    function encodeABA(
        uint256 _value,
        bytes calldata _returnOptions
    ) internal pure returns (bytes memory) {
        return abi.encode(_value, uint16(ABA_TYPE), _returnOptions);
    }

    /// @notice Decodes ABA message format
    function decodeABA(bytes calldata _msg) internal pure returns (
        uint256 value,
        uint16 msgType,
        bytes memory returnOptions
    ) {
        (value, msgType, returnOptions) = abi.decode(_msg, (uint256, uint16, bytes));
    }
}
```

### 2. Update MyOApp Contract

**File: `contracts/MyOApp.sol`**

#### Add new functions:

```solidity
import { OptionsBuilder } from "@layerzerolabs/oapp-evm/contracts/oapp/libs/OptionsBuilder.sol";

contract MyOApp is OApp, OAppOptionsType3 {
    // ... existing code ...

    /**
     * @notice Sends a message with ABA pattern (ping-pong)
     * @param _dstEid Destination endpoint ID
     * @param _returnOptions Options for the return message (B→A)
     * @param _returnGasEstimate Estimated gas cost for return message (B→A)
     * @param _options Additional options for the initial send (A→B)
     * @dev Calculates total gas for A→B + B→A and includes return gas in options
     * @dev User must provide returnGasEstimate - consider using quoteReturnGas() helper
     */
    function sendABA(
        uint32 _dstEid,
        bytes calldata _returnOptions,
        uint128 _returnGasEstimate,
        bytes calldata _options
    ) external payable returns (MessagingReceipt memory receipt) {
        uint256 oldBall = ball;
        ball = ball - 1;
        emit BallServed(msg.sender, _dstEid, oldBall, ball);

        // 1. Quote the return message cost (B→A)
        // NOTE: You cannot directly quote B→A from Chain A. Options:
        // a) User provides returnGasEstimate parameter
        // b) Use a fixed multiplier (e.g., 1.2x of A→B cost)
        // c) Query destination chain via lzRead (complex)
        // For this example, we'll require user to provide estimate
        bytes memory returnMessage = Uint256MsgCodec.encode(ball);
        // Would need to query destination chain or use estimate
        // MessagingFee memory returnFee = _quote(...);

        // 2. Build options with return gas included
        // Base gas for receive + return gas estimate
        bytes memory abaOptions = OptionsBuilder.newOptions()
            .addExecutorLzReceiveOption(
                80000 + _returnGasEstimate,  // Base gas + return gas estimate
                0
            );

        // 3. Encode message with ABA type and return options
        bytes memory _message = Uint256MsgCodec.encodeABA(ball, _returnOptions);
        
        // 4. Combine with caller options
        bytes memory combinedOptions = combineOptions(
            _dstEid,
            Uint256MsgCodec.ABA_TYPE,
            abi.encodePacked(abaOptions, _options)
        );

        // 5. Send with total fee (A→B + B→A)
        receipt = _lzSend(
            _dstEid,
            _message,
            combinedOptions,
            MessagingFee(msg.value, 0),
            payable(msg.sender)
        );
    }

    /**
     * @notice Quotes the cost for ABA pattern (A→B only)
     * @dev User must separately estimate return gas cost
     * @dev Consider using a helper function that queries destination chain
     */
    function quoteABA(
        uint32 _dstEid,
        bytes calldata _returnOptions,
        bytes calldata _options,
        bool _payInLzToken
    ) public view returns (MessagingFee memory sendFee) {
        uint256 tempBall = ball - 1;
        
        // Quote initial send (A→B) with ABA message
        bytes memory abaMessage = Uint256MsgCodec.encodeABA(tempBall, _returnOptions);
        bytes memory options = combineOptions(_dstEid, Uint256MsgCodec.ABA_TYPE, _options);
        sendFee = _quote(_dstEid, abaMessage, options, _payInLzToken);
        
        // Note: Cannot quote B→A from Chain A directly
        // User should:
        // 1. Query destination chain's quoteReturnGas() function
        // 2. Use a fixed multiplier (e.g., 1.2x of A→B)
        // 3. Use lzRead to query destination chain
    }

    /**
     * @notice Helper to estimate return gas cost (for use from destination chain)
     * @dev This would be called on Chain B to help Chain A estimate return costs
     * @dev Or use lzRead to query this from Chain A
     */
    function quoteReturnGas(
        uint32 _srcEid,
        bytes calldata _returnOptions,
        bool _payInLzToken
    ) public view returns (MessagingFee memory returnFee) {
        uint256 tempBall = ball - 1;
        bytes memory returnMessage = Uint256MsgCodec.encode(tempBall);
        returnFee = _quote(_srcEid, returnMessage, _returnOptions, _payInLzToken);
    }

    /**
     * @dev Override _lzReceive to handle ABA pattern
     */
    function _lzReceive(
        Origin calldata _origin,
        bytes32 /*_guid*/,
        bytes calldata payload,
        address _executor,
        bytes calldata /*_extraData*/
    ) internal override {
        // Try to decode as ABA message first
        (uint256 value, uint16 msgType, bytes memory returnOptions) = 
            Uint256MsgCodec.decodeABA(payload);

        uint256 oldBall = ball;
        ball = value;
        emit BallReceived(_executor, oldBall, ball);

        // If ABA type, send response back
        if (msgType == Uint256MsgCodec.ABA_TYPE) {
            // Decrement ball for return message
            ball = ball - 1;
            
            // Encode return message (vanilla type)
            bytes memory returnMessage = Uint256MsgCodec.encode(ball);
            
            // Send back to origin chain
            _lzSend(
                _origin.srcEid,
                returnMessage,
                returnOptions,
                MessagingFee(msg.value, 0),  // Use forwarded value
                payable(address(this))  // Refund to contract
            );
        }
    }
}
```

**Note:** The gas forwarding mechanism needs careful handling. The `msg.value` in `_lzReceive` contains the forwarded gas, but you need to ensure it's sufficient for the return message.

---

## Solana Program Changes

### 1. Update Message Codec

**File: `programs/my_oapp/src/uint256_msg_codec.rs`**

Add ABA encoding/decoding:

```rust
pub const ABA_TYPE: u16 = 2;

pub struct AbaMessage {
    pub ball: [u8; 32],
    pub msg_type: u16,
    pub return_options: Vec<u8>,
}

pub fn encode_aba(ball: &[u8; 32], return_options: &[u8]) -> Vec<u8> {
    // Encode: ball (32 bytes) + msg_type (2 bytes) + return_options (variable)
    let mut encoded = Vec::new();
    encoded.extend_from_slice(ball);
    encoded.extend_from_slice(&(ABA_TYPE as u16).to_be_bytes());
    encoded.extend_from_slice(&(return_options.len() as u32).to_be_bytes()); // length prefix
    encoded.extend_from_slice(return_options);
    encoded
}

pub fn decode_aba(message: &[u8]) -> Result<AbaMessage> {
    require!(message.len() >= 34, MyOAppError::InvalidMessageLength);
    
    let mut ball = [0u8; 32];
    ball.copy_from_slice(&message[0..32]);
    
    let msg_type = u16::from_be_bytes([message[32], message[33]]);
    
    if message.len() > 34 {
        let options_len = u32::from_be_bytes([
            message[34], message[35], message[36], message[37]
        ]) as usize;
        require!(message.len() >= 38 + options_len, MyOAppError::InvalidMessageLength);
        
        let return_options = message[38..38 + options_len].to_vec();
        Ok(AbaMessage { ball, msg_type, return_options })
    } else {
        Ok(AbaMessage { 
            ball, 
            msg_type, 
            return_options: Vec::new() 
        })
    }
}
```

### 2. Update Send Instruction

**File: `programs/my_oapp/src/instructions/send.rs`**

Add ABA send function:

```rust
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct SendAbaMessageParams {
    pub dst_eid: u32,
    pub return_options: Vec<u8>,  // Options for B→A return
    pub options: Vec<u8>,          // Options for A→B send
    pub native_fee: u64,
    pub lz_token_fee: u64,
}

// New instruction handler for ABA
pub fn send_aba(ctx: &mut Context<Send>, params: &SendAbaMessageParams) -> Result<()> {
    let store = &mut ctx.accounts.store;
    let ball = store.ball;
    let ball_ethnum = U256::from_be_bytes(ball);
    let new_ball_ethnum = ball_ethnum.saturating_sub(U256::ONE);
    let new_ball = new_ball_ethnum.to_be_bytes();
    
    // Encode ABA message
    let message = uint256_msg_codec::encode_aba(&new_ball, &params.return_options);
    
    // Build options with return gas included
    // (Similar to EVM OptionsBuilder logic)
    
    // Emit event
    emit!(crate::events::BallSent { /* ... */ });
    
    // Send via Endpoint CPI
    let send_params = SendParams {
        dst_eid: params.dst_eid,
        receiver: ctx.accounts.peer.peer_address,
        message,
        options: ctx.accounts.peer.enforced_options
            .combine_options(&None::<Vec<u8>>, &params.options)?,
        native_fee: params.native_fee,
        lz_token_fee: params.lz_token_fee,
    };
    
    oapp::endpoint_cpi::send(/* ... */)?;
    Ok(())
}
```

### 3. Update LzReceive Instruction

**File: `programs/my_oapp/src/instructions/lz_receive.rs`**

Modify to handle ABA pattern:

```rust
impl LzReceive<'_> {
    pub fn apply(ctx: &mut Context<LzReceive>, params: &LzReceiveParams) -> Result<()> {
        // ... existing clear logic ...

        // Try to decode as ABA message
        match uint256_msg_codec::decode_aba(&params.message) {
            Ok(aba_msg) => {
                // Update ball
                let store = &mut ctx.accounts.store;
                let old_ball = store.ball;
                store.set_ball(aba_msg.ball);
                
                // If ABA type, send response back
                if aba_msg.msg_type == uint256_msg_codec::ABA_TYPE {
                    // Decrement ball
                    let ball_ethnum = U256::from_be_bytes(aba_msg.ball);
                    let new_ball_ethnum = ball_ethnum.saturating_sub(U256::ONE);
                    let new_ball = new_ball_ethnum.to_be_bytes();
                    
                    // Encode return message (vanilla)
                    let return_message = uint256_msg_codec::encode(&new_ball);
                    
                    // Get return peer config (for src_eid)
                    // This requires looking up the peer config for params.src_eid
                    // Then send back using Endpoint CPI
                    
                    // Note: Need to handle gas forwarding - the native_fee from params
                    // should be used for the return message
                }
            }
            Err(_) => {
                // Fallback to vanilla decode
                let ball = uint256_msg_codec::decode(&params.message)?;
                // ... existing logic ...
            }
        }
        
        Ok(())
    }
}
```

**Challenge:** In Solana, you need to:
1. Look up the peer config for `params.src_eid` to get return address
2. Handle gas forwarding (the `native_fee` in the received message)
3. Make a CPI call to send the return message

---

## Gas Forwarding Considerations

### Critical Challenge: Estimating Return Gas Cost

**Problem:** You cannot directly quote the cost of B→A from Chain A because:
- Gas prices differ between chains
- Network conditions vary
- You need destination chain's current state

**Solutions:**

1. **User-Provided Estimate (Simplest)**
   - User calls `quoteReturnGas()` on destination chain first
   - Passes estimate as parameter to `sendABA()`
   - Pros: Accurate, explicit
   - Cons: Requires extra transaction/query

2. **Fixed Multiplier**
   - Use 1.2x - 1.5x of A→B cost as estimate
   - Pros: Simple, no extra calls
   - Cons: May over/under-estimate

3. **lzRead Query (Most Accurate)**
   - Use LayerZero's lzRead to query destination chain
   - Call `quoteReturnGas()` on destination from source
   - Pros: Most accurate
   - Cons: More complex, requires lzRead setup

### EVM Gas Forwarding
- The `msg.value` in `_lzReceive` contains forwarded gas
- Use `MessagingFee(msg.value, 0)` when calling `_lzSend` for return
- **Important:** The forwarded value comes from the `ExecutorLzReceiveOption` in the original message
- Ensure sufficient value is forwarded in the A→B options

### Solana Gas Forwarding
- The `native_fee` in `LzReceiveParams` contains forwarded gas
- Use this fee when making the return `send` CPI call
- May need to adjust based on actual return message cost
- **Important:** Solana's fee model is different - ensure sufficient lamports are forwarded

---

## Testing Requirements

1. **Unit Tests:**
   - Test ABA message encoding/decoding
   - Test gas calculation for both legs
   - Test return message sending

2. **Integration Tests:**
   - Test full A→B→A flow
   - Test gas forwarding accuracy
   - Test with insufficient gas scenarios

3. **Edge Cases:**
   - What happens if return message fails?
   - What if return options are malformed?
   - What if gas is insufficient for return?

---

## Implementation Checklist

### EVM Contract
- [ ] Add `ABA_TYPE` constant to `Uint256MsgCodec`
- [ ] Implement `encodeABA()` and `decodeABA()` functions
- [ ] Add `sendABA()` function with gas calculation
- [ ] Add `quoteABA()` function
- [ ] Modify `_lzReceive()` to handle ABA pattern
- [ ] Add events for ABA sends/receives
- [ ] Update tests

### Solana Program
- [ ] Add ABA encoding/decoding to `uint256_msg_codec.rs`
- [ ] Add `SendAbaMessageParams` struct
- [ ] Implement `send_aba` instruction handler
- [ ] Modify `lz_receive` to detect and handle ABA messages
- [ ] Implement return message sending logic
- [ ] Handle peer config lookup for return address
- [ ] Update tests

### SDK/Client Updates
- [ ] Update TypeScript client to support ABA sends
- [ ] Add helper functions for gas calculation
- [ ] Update deployment/wiring scripts if needed

---

## Important Notes

1. **Gas Estimation:** The return message gas cost must be estimated BEFORE sending the initial message. This may require:
   - Querying the destination chain's current gas prices
   - Using a fixed estimate (less accurate but simpler)
   - Implementing a two-phase quote system

2. **Error Handling:** If the return message fails, the initial message has already been processed. Consider:
   - Revert mechanisms (if possible)
   - Event logging for failed returns
   - Retry mechanisms

3. **Message Type Compatibility:** Ensure backward compatibility with existing vanilla messages. The decode function should handle both formats.

4. **Options Encoding:** The `returnOptions` need to be properly encoded/decoded and passed through the message payload. Consider size limits and validation.

