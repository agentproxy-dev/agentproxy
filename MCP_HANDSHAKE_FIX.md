# MCP Handshake Fix: Missing `initialized` Notification

## Background Context
We could not get streamable_http working, even though we see in [PR-116](https://github.com/agentgateway/agentgateway/pull/116/files) addressing that, so we defaulted to SSE. We could only use config.json + UI to use stdio | sse | openai for targets, but we could not get our MCP Servers to connect in agentgateway's UI. 


## Problem Statement

The agentgateway was experiencing a race condition with MCP (Model Context Protocol) backend servers, where requests like `tools/list` would fail with errors such as:

```
"Failed to validate request: Received request before initialization was complete."
```

This issue occurred because backend MCP servers were not considering the handshake complete, even after long delays, while tools like MCP Inspector worked instantly.

## Root Cause Analysis

After investigating the MCP protocol specification and comparing with working implementations (like MCP Inspector), we discovered that agentgateway was **only sending the `initialize` request** but was **missing the crucial `initialized` notification** required to complete the MCP handshake.

### MCP Protocol Handshake Sequence

The correct MCP handshake sequence is:

1. **Client** → **Server**: `initialize` request
2. **Server** → **Client**: `initialize` response  
3. **Client** → **Server**: `initialized` notification ← **THIS WAS MISSING**
4. Now the client can send other requests like `tools/list`

### What Was Happening

- ✅ agentgateway was sending `initialize` requests
- ✅ agentgateway was receiving `initialize` responses  
- ❌ agentgateway was **NOT** sending `initialized` notifications
- ❌ Backend MCP servers never considered the handshake complete
- ❌ Subsequent requests were rejected with "initialization not complete" errors

## Solution

### Backend Changes

#### 1. Modified `upstream.rs` - Added `initialized` Notification

In `crates/agentgateway/src/relay/upstream.rs`, the `initialize()` method now:

1. Sends the `initialize` request (as before)
2. Waits for and validates the response (as before)  
3. **NEW**: Sends an `initialized` notification to complete the handshake

```rust
// Send the initialized notification to complete the handshake
tracing::debug!("sending initialized notification to MCP target");
if let Err(e) = m.send_notification(ClientNotification::InitializedNotification(InitializedNotification {
    method: Default::default(),
    extensions: rmcp::model::Extensions::new(),
})).await {
    tracing::warn!("failed to send initialized notification: {}", e);
    // Don't fail the connection for this, but log it
} else {
    tracing::debug!("initialized notification sent successfully");
}
```

#### 2. Modified `pool.rs` - Initialize Connections During Establishment

In `crates/agentgateway/src/relay/pool.rs`, connections are now initialized immediately after being established:

- Added `initialize()` call for both SSE and stdio transports during connection setup
- Added proper error handling with descriptive error messages
- Added debug logging to track initialization progress

#### 3. Modified `relay.rs` - Updated Relay Initialization Logic  

In `crates/agentgateway/src/relay.rs`, the relay's `initialize()` method now:

- Acknowledges that connections are pre-initialized during establishment
- Simply marks existing connections as ready instead of re-initializing them

## Impact

### Before Fix
- ❌ MCP handshake incomplete - missing `initialized` notification
- ❌ Backend servers reject requests with "initialization not complete"  
- ❌ Race conditions and timing-dependent failures
- ❌ Long delays don't help because handshake never completes

### After Fix  
- ✅ Complete MCP handshake with proper `initialized` notification
- ✅ Backend servers accept requests immediately after handshake
- ✅ No more "initialization not complete" errors
- ✅ Works instantly like MCP Inspector and other compliant clients

## Testing

To verify the fix works:

1. **Start agentgateway** with any MCP server + SSE configuration
2. **Connect via UI** or API to list tools
3. **Check logs** for successful initialization sequence:
   ```
   sending initialize request to MCP target
   MCP target initialization successful  
   sending initialized notification to MCP target
   initialized notification sent successfully
   ```
4. **Verify** that `tools/list` and other requests work immediately without errors

## Files Modified

- `crates/agentgateway/src/relay/upstream.rs` - Added `initialized` notification
- `crates/agentgateway/src/relay/pool.rs` - Initialize connections during establishment  
- `crates/agentgateway/src/relay.rs` - Updated relay initialization logic

## Compliance

This fix brings agentgateway into compliance with the MCP protocol specification by ensuring the complete handshake sequence is followed, matching the behavior of other working MCP clients like MCP Inspector.
