# Avatar Sync Investigation

## Summary

Investigation into missing contact avatars in Signal Tauri application.

## Current Status

✅ **UI Fixes Applied**
- Chat view header now uses avatar_cache for proper avatar loading
- Chat list already had avatar support
- Fallback to colored circles with initials working correctly

❌ **Avatars Not Being Synced**
- Avatar directory is empty: `~/Library/Application Support/org.signal-tauri.Signal/avatars/`
- No .jpg files downloaded despite having contacts with profile pictures

## How Avatar Sync Works

### Code Flow

1. **Initial Sync** (during device linking)
   - `src/signal/manager.rs:552`: `sync_contact_avatars()` is called after contact sync
   - This only happens once during the SyncCompleted event

2. **Avatar Fetching** (`src/signal/profiles.rs`)
   ```rust
   sync_contact_avatars(&mut manager, storage)
     ↓
   For each contact with profile_key:
     fetch_and_save_avatar(manager, uuid, profile_key, avatars_dir)
       ↓
     Download from Signal servers
       ↓
     Save to: {avatars_dir}/{uuid}.jpg
       ↓
     Update contact.avatar_path in database
   ```

3. **Avatar Display**
   - Chat list: `src/ui/views/chat_list.rs:407` - uses `AvatarCache::get_or_load()`
   - Chat view header: `src/ui/views/chat_view.rs:312` - uses `draw_avatar()` (FIXED)
   - Both fall back to colored circles + initials if no image

### Storage Locations

- **Avatar files**: `~/Library/Application Support/org.signal-tauri.Signal/avatars/{uuid}.jpg`
- **Avatar paths**: Stored in SQLite database fields:
  - `contacts.avatar_path`
  - `conversations.avatar_path`

## Current Behavior

### What's Working ✅

1. Avatars from primary Signal app ARE displayed (those that exist in the chat list)
2. Avatar caching system works
3. Fallback to initials works
4. UI properly requests and displays avatars

### What's NOT Working ❌

1. Avatars directory is empty (0 files)
2. Some contacts show avatars (from where?), others don't
3. Avatar sync only happens once during initial device linking

## Possible Issues

### Issue 1: Profile Keys Missing

**Hypothesis**: Contacts in database don't have `profile_key` set.

**Check**:
```sql
SELECT uuid, display_name, profile_key FROM contacts;
```

If `profile_key` is NULL, avatars can't be fetched (requires encryption key).

### Issue 2: Avatar Sync Failed Silently

**Evidence**: No logs about avatar fetching in terminal output.

**Possible Causes**:
- Network error during fetch
- Permission error writing to avatars directory
- Profile key decryption failure
- Signal server returned no avatar data

**Expected Logs** (not seen):
```
INFO: Starting avatar sync for contacts...
INFO: Avatar sync complete: X avatars fetched
```

### Issue 3: Avatars Coming From Elsewhere

**Observation**: Some contacts (保重的小龍, Bryan chk, etc.) DO show avatars in the chat list.

**Question**: Where are these images coming from if `avatars/` directory is empty?

**Possibilities**:
1. Cached in different location?
2. Embedded in sync message?
3. Being fetched on-demand without saving?
4. Test data/hardcoded?

### Issue 4: One-Time Sync Only

**Current Behavior**: Avatar sync only happens during `SyncCompleted` event (device linking).

**Problem**:
- If sync fails or user links device when offline, avatars never get fetched
- No retry mechanism
- No manual refresh option

## Recommendations

### Immediate Actions

1. **Add Debug Logging**
   ```rust
   // In profiles.rs:sync_contact_avatars()
   tracing::info!("Found {} contacts to sync", contacts_with_keys.len());
   tracing::info!("Attempting avatar fetch for {}: {}", uuid, display_name);
   ```

2. **Check Database**
   ```bash
   sqlite3 ~/Library/Application\ Support/org.signal-tauri.Signal/signal_protocol.db \
     "SELECT COUNT(*) FROM contacts WHERE profile_key IS NOT NULL;"
   ```

3. **Check Permissions**
   ```bash
   ls -ld ~/Library/Application\ Support/org.signal-tauri.Signal/avatars/
   ```

### Long-Term Fixes

1. **Add Manual Refresh**
   - UI button to trigger `sync_contact_avatars()`
   - Menu option: "Refresh Contact Avatars"

2. **Add Periodic Sync**
   ```rust
   // Every 1 hour or on app startup
   if last_avatar_sync > 1.hour.ago() {
       sync_contact_avatars(...).await;
   }
   ```

3. **Add Avatar Sync to Individual Profile Fetch**
   - When viewing conversation, fetch avatar for that specific contact
   - Don't rely only on bulk sync

4. **Better Error Handling**
   ```rust
   match fetch_and_save_avatar(...).await {
       Ok(path) => tracing::info!("Avatar saved: {}", path),
       Err(e) => tracing::warn!("Avatar fetch failed for {}: {}", uuid, e),
   }
   ```

5. **Add Retry Logic**
   ```rust
   // Retry failed avatars after X minutes
   for contact in contacts_with_failed_avatars() {
       retry_avatar_fetch(contact).await;
   }
   ```

## Testing Steps

### Verify Avatar Sync Runs

1. Delete database to force re-linking:
   ```bash
   rm -rf ~/Library/Application\ Support/org.signal-tauri.Signal/
   ```

2. Run app with logging:
   ```bash
   RUST_LOG=debug cargo run --release 2>&1 | tee avatar_sync.log
   ```

3. Link device and check logs for:
   - "Starting avatar sync for contacts..."
   - "Avatar sync complete: X avatars fetched"
   - Any errors during fetch

4. Check avatars directory:
   ```bash
   ls -lh ~/Library/Application\ Support/org.signal-tauri.Signal/avatars/
   ```

### Verify Database

```bash
sqlite3 ~/Library/Application\ Support/org.signal-tauri.Signal/signal_protocol.db <<EOF
.headers on
SELECT uuid, display_name, profile_key, avatar_path FROM contacts LIMIT 10;
EOF
```

Expected:
- `profile_key` should be a base64 string (32 bytes encoded)
- `avatar_path` should be `/path/to/avatars/{uuid}.jpg` if synced

## Related Files

- `src/signal/profiles.rs` - Avatar fetching logic
- `src/signal/manager.rs:552` - Avatar sync trigger
- `src/ui/avatar_cache.rs` - Avatar caching and display
- `src/ui/views/chat_list.rs:407` - Chat list avatar rendering
- `src/ui/views/chat_view.rs:312` - Chat view header avatar rendering
- `src/storage/contacts.rs` - Contact storage with avatar_path
- `src/storage/conversations.rs` - Conversation storage with avatar_path

## Next Steps

1. ✅ UI fixes completed (header avatar now works)
2. ⏳ Run app with debug logging to capture avatar sync behavior
3. ⏳ Investigate where existing avatars are coming from
4. ⏳ Add manual refresh button for testing
5. ⏳ Implement robust avatar sync with retry logic
