-- SQL Script to Clean Up Invalid Conversations
-- This script removes conversations with invalid IDs or problematic data
--
-- IMPORTANT: Backup your database before running this!
-- Location: ~/Library/Application Support/org.signal-tauri.Signal/app.db
--
-- To run this script from within your application or using the encrypted connection

-- First, let's see what we're going to delete
SELECT
    id,
    conversation_type,
    name,
    (SELECT COUNT(*) FROM messages WHERE conversation_id = conversations.id) as message_count,
    created_at,
    updated_at
FROM conversations
WHERE
    -- Invalid numeric IDs (should be UUIDs)
    id IN ('1', '2', '3', '23', '24')
    OR
    -- Conversation with special Unicode tag characters
    id = '6cf1d9af-96d7-40bc-9fc6-a752244d79c4';

-- If you're comfortable with deleting the above, uncomment the following:

-- Delete messages associated with these conversations first
-- DELETE FROM messages
-- WHERE conversation_id IN (
--     '1', '2', '3', '23', '24',
--     '6cf1d9af-96d7-40bc-9fc6-a752244d79c4'
-- );

-- Then delete the conversations themselves
-- DELETE FROM conversations
-- WHERE
--     id IN ('1', '2', '3', '23', '24')
--     OR
--     id = '6cf1d9af-96d7-40bc-9fc6-a752244d79c4';

-- Optional: Update the conversation with Unicode tag characters instead of deleting
-- This renames it to something more visible
-- UPDATE conversations
-- SET name = '[Invalid Unicode Name - Please Update]'
-- WHERE id = '6cf1d9af-96d7-40bc-9fc6-a752244d79c4';

-- Verify the cleanup
-- SELECT
--     COUNT(*) as remaining_invalid_conversations
-- FROM conversations
-- WHERE
--     id IN ('1', '2', '3', '23', '24')
--     OR
--     id = '6cf1d9af-96d7-40bc-9fc6-a752244d79c4';
