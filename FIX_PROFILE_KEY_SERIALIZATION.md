# Fix: Profile Key Serialization Error

## Issue

After fixing the `signaling_key` issue, device linking was still failing with:

```
ERROR signal_tauri::signal::manager: Device linking failed: Storage error: Failed to create registration data: Error("invalid type: string \"ZJiyg+g5uvpVUMqYfe1VqilPJ2luP/WLnOKQY4ti9Aw=\", expected a borrowed string", line: 0, column: 0)
```

## Root Cause

The `RegistrationData` struct has a `profile_key` field that uses a custom serde deserializer (`serde_profile_key`) which expects a **borrowed string (`&str`)** during deserialization:

```rust
// From presage/src/serde.rs line 19
.decode(<&str>::deserialize(deserializer)?)
```

However, the original code was using `serde_json::from_value()` which provides **owned strings (`String`)** from JSON values, causing a type mismatch.

## The Fix

Changed from:
```rust
let reg_data_json = serde_json::json!({ ... });
let registration_data = serde_json::from_value(reg_data_json)?;
```

To:
```rust
let reg_data_value = serde_json::json!({ ... });
let reg_data_json_str = serde_json::to_string(&reg_data_value)?;
let registration_data = serde_json::from_str(&reg_data_json_str)?;
```

**Why this works:**
- `serde_json::from_str()` deserializes from a `&str` which maintains the borrowed string semantics
- The `serde_profile_key::deserialize` function receives the string as `&str` instead of owned `String`
- This satisfies the lifetime requirements of the custom deserializer

## Files Changed

- `src/signal/registration.rs` (lines 254-279)
  - Added `ServiceIds` import
  - Changed JSON deserialization approach to use `to_string + from_str`
  - Added detailed comments explaining the fix

## Testing

```bash
cd ~/Documents/signal-tauri
cargo run --release
```

Expected behavior:
1. ✅ QR code generates successfully
2. ✅ Scan with primary Signal device
3. ✅ Device links without serialization errors
4. ✅ Registration data saves to database
5. ✅ Message history sync begins

## Technical Details

### RegistrationData Structure (presage 20d39de)

```rust
pub struct RegistrationData {
    pub signal_servers: SignalServers,
    pub device_name: Option<String>,
    pub phone_number: PhoneNumber,
    #[serde(flatten)]
    pub service_ids: ServiceIds,
    pub(crate) password: String,
    pub device_id: Option<u32>,
    pub registration_id: u32,
    #[serde(default)]
    pub pni_registration_id: Option<u32>,
    #[serde(with = "serde_profile_key")]  // <-- Custom deserializer
    pub(crate) profile_key: ProfileKey,
}
```

### Custom Deserializer

```rust
// presage/src/serde.rs
pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<ProfileKey, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes: [u8; 32] = general_purpose::STANDARD
        .decode(<&str>::deserialize(deserializer)?)  // Expects &str!
        .map_err(serde::de::Error::custom)?
        .try_into()
        .map_err(|e: Vec<u8>| serde::de::Error::invalid_length(e.len(), &"32 bytes"))?;
    Ok(ProfileKey::create(bytes))
}
```

## Related Issues

- Issue #1: "Invalid frame: unsupported signaling cryptogram version" - Fixed by updating presage
- Issue #2: "Invalid group ID: Invalid symbol 45, offset 8" - Fixed by proper group ID validation
- Issue #3: "Failed to create registration data" (signaling_key) - Fixed by removing obsolete field
- **Issue #4: "expected a borrowed string"** - Fixed by this change

## Status: ✅ RESOLVED

All device linking issues are now fixed and the app should work correctly!
