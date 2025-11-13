import { ethers } from 'ethers'

const TX_HASH = '0x6e85c8eed8373bed30cd0ddc3a5f7af71592a759b84cbb50c1fda3c51839fff0'
const RPC_URL = 'https://eth-sepolia.g.alchemy.com/v2/UgBZILPWmtdXrICFmC7r_kmYMukRa0se'

async function inspectTransaction() {
    const provider = new ethers.providers.JsonRpcProvider(RPC_URL)
    
    console.log('Fetching transaction...')
    const tx = await provider.getTransaction(TX_HASH)
    
    if (!tx) {
        console.error('Transaction not found')
        return
    }
    
    console.log('\n=== Transaction Details ===')
    console.log('From:', tx.from)
    console.log('To:', tx.to)
    console.log('Value:', tx.value.toString())
    console.log('Data length:', tx.data?.length || 0)
    console.log('Data:', tx.data)
    
    // Get receipt to see logs
    const receipt = await provider.getTransactionReceipt(TX_HASH)
    if (!receipt) {
        console.error('Transaction receipt not found')
        return
    }
    
    console.log('\n=== Transaction Receipt ===')
    console.log('Status:', receipt.status === 1 ? 'Success' : 'Failed')
    console.log('Gas used:', receipt.gasUsed.toString())
    console.log('Logs count:', receipt.logs.length)
    
    // Decode the transaction input
    if (tx.data && tx.data.length > 10) {
        console.log('\n=== Decoding Transaction Input ===')
        const iface = new ethers.utils.Interface([
            'function send(uint32 _dstEid, bytes _returnOptions, uint128 _returnGasEstimate, bytes _options)'
        ])
        
        try {
            const decoded = iface.decodeFunctionData('send', tx.data)
            console.log('dstEid:', decoded._dstEid.toString())
            console.log('returnOptions length:', decoded._returnOptions.length / 2 - 1, 'bytes')
            console.log('returnOptions:', decoded._returnOptions)
            console.log('returnGasEstimate:', decoded._returnGasEstimate.toString())
            console.log('options length:', decoded._options.length / 2 - 1, 'bytes')
            console.log('options:', decoded._options)
        } catch (e: any) {
            console.log('Could not decode as send function:', e?.message || e)
        }
    }
    
    // Try to find LayerZero message in logs
    console.log('\n=== Looking for LayerZero Events ===')
    
    // LayerZero PacketSent event signature
    const packetSentIface = new ethers.utils.Interface([
        'event PacketSent(bytes encodedPayload, bytes options, address sendLibrary)'
    ])
    
    for (let i = 0; i < receipt.logs.length; i++) {
        const log = receipt.logs[i]
        console.log(`\nLog ${i}:`)
        console.log('  Address:', log.address)
        console.log('  Topics:', log.topics.length)
        console.log('  Data length:', log.data.length)
        
        // Try to decode as PacketSent event
        try {
            const decoded = packetSentIface.parseLog(log)
            console.log('  ✓ Decoded as PacketSent event')
            console.log('  encodedPayload length:', decoded.args.encodedPayload.length / 2 - 1, 'bytes')
            console.log('  options length:', decoded.args.options.length / 2 - 1, 'bytes')
            console.log('  sendLibrary:', decoded.args.sendLibrary)
            
            // Now decode the encodedPayload to get the actual message
            const payloadBytes = ethers.utils.arrayify(decoded.args.encodedPayload)
            console.log('  Payload bytes length:', payloadBytes.length)
            
            // The encodedPayload contains the actual ABA message
            if (payloadBytes.length >= 128) {
                console.log('\n  === Decoding ABA Message from encodedPayload ===')
                // Decode ABA message: abi.encode(uint256, uint16, bytes)
                const ballBytes = payloadBytes.slice(0, 32)
                const ball = ethers.BigNumber.from(ballBytes)
                console.log('  Ball (uint256):', ball.toString())
                console.log('  Ball (hex):', ethers.utils.hexlify(ballBytes))
                
                const msgTypeBytes = payloadBytes.slice(32, 64)
                const msgType = ethers.BigNumber.from(msgTypeBytes.slice(30, 32))
                console.log('  Message Type (uint16):', msgType.toString())
                console.log('  Expected ABA_TYPE: 2')
                
                if (msgType.toString() !== '2') {
                    console.log('  ⚠️  ERROR: Message type is', msgType.toString(), 'but expected 2!')
                }
                
                const offsetBytes = payloadBytes.slice(64, 96)
                const offset = ethers.BigNumber.from(offsetBytes.slice(24, 32))
                console.log('  Return Options Offset:', offset.toString())
                
                if (offset.gte(128) && offset.lte(payloadBytes.length)) {
                    const offsetNum = Number(offset.toString())
                    const lenBytes = payloadBytes.slice(offsetNum, offsetNum + 32)
                    const len = ethers.BigNumber.from(lenBytes.slice(24, 32))
                    console.log('  Return Options Length:', len.toString())
                    
                    if (len.gt(0)) {
                        const lenNum = Number(len.toString())
                        const returnOptionsBytes = payloadBytes.slice(offsetNum + 32, offsetNum + 32 + lenNum)
                        console.log('  Return Options:', ethers.utils.hexlify(returnOptionsBytes))
                    } else {
                        console.log('  Return Options: (empty)')
                    }
                }
            }
            continue
        } catch (e: any) {
            // Not a PacketSent event, continue
        }
        
        // Decode the log data - LayerZero PacketSent event: PacketSent(bytes encodedPayload, bytes options, address sendLibrary)
        // The data field contains ABI-encoded bytes for encodedPayload and options
        if (log.data && log.data.length > 66) {
            console.log('  Full Data length:', log.data.length / 2 - 1, 'bytes')
            console.log('  Full Data (first 300 chars):', log.data.substring(0, 300) + '...')
            
            // Decode the ABI-encoded data
            // PacketSent(bytes encodedPayload, bytes options, address sendLibrary)
            // The data is ABI-encoded, so we need to decode it
            const dataBytes = ethers.utils.arrayify(log.data)
            
            // ABI encoding for dynamic bytes: offset (32 bytes) + length (32 bytes) + data
            // First, find the offset to encodedPayload
            if (dataBytes.length >= 32) {
                const payloadOffset = ethers.BigNumber.from(dataBytes.slice(0, 32))
                console.log('  Payload offset:', payloadOffset.toString())
                
                if (payloadOffset.gte(96) && dataBytes.length >= Number(payloadOffset.toString()) + 32) {
                    const payloadOffsetNum = Number(payloadOffset.toString())
                    const payloadLength = ethers.BigNumber.from(dataBytes.slice(payloadOffsetNum, payloadOffsetNum + 32))
                    const payloadLengthNum = Number(payloadLength.toString())
                    console.log('  Payload length:', payloadLengthNum, 'bytes')
                    
                    if (dataBytes.length >= payloadOffsetNum + 32 + payloadLengthNum) {
                        const actualPayload = dataBytes.slice(payloadOffsetNum + 32, payloadOffsetNum + 32 + payloadLengthNum)
                        console.log('  Actual Payload (hex, first 200 chars):', ethers.utils.hexlify(actualPayload).substring(0, 200) + '...')
                        
                        // Now decode the ABA message from the actual payload
                        if (actualPayload.length >= 128) {
                            console.log('\n  === Decoding ABA Message Payload ===')
                            console.log('  Payload bytes length:', actualPayload.length)
                            
                            // First 32 bytes: uint256 (ball)
                            const ballBytes = actualPayload.slice(0, 32)
                            const ball = ethers.BigNumber.from(ballBytes)
                            console.log('  Ball (uint256):', ball.toString())
                            console.log('  Ball (hex):', ethers.utils.hexlify(ballBytes))
                            
                            // Next 32 bytes: uint16 padded (msg_type)
                            const msgTypeBytes = actualPayload.slice(32, 64)
                            const msgType = ethers.BigNumber.from(msgTypeBytes.slice(30, 32))
                            console.log('  Message Type (uint16):', msgType.toString())
                            console.log('  Expected ABA_TYPE: 2')
                            console.log('  Message Type bytes (hex):', ethers.utils.hexlify(msgTypeBytes))
                            
                            // Next 32 bytes: offset to return_options (uint256, but we read as u64 from last 8 bytes)
                            const offsetBytes = actualPayload.slice(64, 96)
                            console.log('  Offset bytes (hex):', ethers.utils.hexlify(offsetBytes))
                            const offset = ethers.BigNumber.from(offsetBytes.slice(24, 32))
                            console.log('  Return Options Offset (u64):', offset.toString())
                            
                            // Validate offset is reasonable (should be 128 for empty return_options)
                            if (offset.gte(128) && offset.lte(actualPayload.length)) {
                                const offsetNum = Number(offset.toString())
                                if (actualPayload.length >= offsetNum + 32) {
                                    // Length of return_options (uint256, read as u64 from last 8 bytes)
                                    const lenBytes = actualPayload.slice(offsetNum, offsetNum + 32)
                                    const len = ethers.BigNumber.from(lenBytes.slice(24, 32))
                                    console.log('  Return Options Length:', len.toString())
                                    
                                    if (len.gt(0) && actualPayload.length >= offsetNum + 32 + Number(len.toString())) {
                                        const lenNum = Number(len.toString())
                                        const returnOptionsBytes = actualPayload.slice(offsetNum + 32, offsetNum + 32 + lenNum)
                                        console.log('  Return Options:', ethers.utils.hexlify(returnOptionsBytes))
                                    } else {
                                        console.log('  Return Options: (empty)')
                                    }
                                } else {
                                    console.log('  ERROR: Payload too short for offset', offsetNum)
                                }
                            } else {
                                console.log('  ERROR: Invalid offset value:', offset.toString())
                                console.log('  Expected offset >= 128 and <=', actualPayload.length)
                            }
                        } else {
                            console.log('  ERROR: Payload too short for ABA format (need at least 128 bytes, got', actualPayload.length, ')')
                        }
                    }
                }
            }
        }
    }
    
    // Try to get the actual message payload from LayerZero
    // This would require querying the LayerZero endpoint contract
    console.log('\n=== Checking LayerZero Endpoint ===')
    console.log('To inspect the actual message payload, you may need to:')
    console.log('1. Query the LayerZero Endpoint contract for the message')
    console.log('2. Check LayerZeroScan for message details')
    console.log('3. Use the LayerZero SDK to decode the message')
}

inspectTransaction().catch(console.error)

