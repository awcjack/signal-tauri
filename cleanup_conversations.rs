/// Database Cleanup Utility for Signal Tauri
///
/// This tool cleans up invalid conversations from the database.
///
/// To use:
/// 1. Add this as a binary in Cargo.toml:
///    [[bin]]
///    name = "cleanup"
///    path = "cleanup_conversations.rs"
///
/// 2. Run with: cargo run --bin cleanup
///
/// Or integrate this code into your main application as a one-time migration.

use rusqlite::{Connection, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    // Get the database path
    let app_support = std::env::var("HOME")
        .map(PathBuf::from)
        .expect("Could not find home directory")
        .join("Library")
        .join("Application Support")
        .join("org.signal-tauri.Signal");

    let db_path = app_support.join("app.db");
    let key_path = app_support.join(".encryption_key");

    // Read the encryption key
    let key = std::fs::read_to_string(&key_path)
        .expect("Could not read encryption key");

    // Open encrypted database
    let mut conn = Connection::open(&db_path)?;

    // Set the encryption key
    conn.pragma_update(None, "key", &key)?;

    println!("=== Signal Tauri Database Cleanup ===\n");

    // First, show what we're going to clean up
    println!("Scanning for invalid conversations...\n");

    let mut stmt = conn.prepare(
        "SELECT id, conversation_type, name,
                (SELECT COUNT(*) FROM messages WHERE conversation_id = conversations.id) as message_count
         FROM conversations
         WHERE id IN ('1', '2', '3', '23', '24')
            OR id = '6cf1d9af-96d7-40bc-9fc6-a752244d79c4'"
    )?;

    let invalid_convs: Vec<(String, String, String, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
            ))
        })?
        .collect::<Result<Vec<_>>>()?;

    // Drop statement to release the borrow before transaction
    drop(stmt);

    if invalid_convs.is_empty() {
        println!("No invalid conversations found. Database is clean!");
        return Ok(());
    }

    println!("Found {} invalid conversation(s):\n", invalid_convs.len());

    for (id, conv_type, name, msg_count) in &invalid_convs {
        println!("  ID: {}", id);
        println!("  Type: {}", conv_type);
        println!("  Name: {}", name);
        println!("  Messages: {}", msg_count);
        println!();
    }

    // Ask for confirmation
    println!("Do you want to delete these conversations and their messages? (yes/no)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Failed to read input");

    if input.trim().to_lowercase() != "yes" {
        println!("Cleanup cancelled.");
        return Ok(());
    }

    // Begin transaction
    let tx = conn.transaction()?;

    // Delete messages first (foreign key constraint)
    let deleted_messages = tx.execute(
        "DELETE FROM messages
         WHERE conversation_id IN ('1', '2', '3', '23', '24', '6cf1d9af-96d7-40bc-9fc6-a752244d79c4')",
        [],
    )?;

    println!("\nDeleted {} messages", deleted_messages);

    // Delete conversations
    let deleted_convs = tx.execute(
        "DELETE FROM conversations
         WHERE id IN ('1', '2', '3', '23', '24')
            OR id = '6cf1d9af-96d7-40bc-9fc6-a752244d79c4'",
        [],
    )?;

    println!("Deleted {} conversations", deleted_convs);

    // Commit transaction
    tx.commit()?;

    println!("\n✓ Cleanup completed successfully!");

    // Verify
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM conversations
         WHERE id IN ('1', '2', '3', '23', '24')
            OR id = '6cf1d9af-96d7-40bc-9fc6-a752244d79c4'",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        println!("✓ Verification passed: No invalid conversations remain");
    } else {
        println!("⚠ Warning: {} invalid conversations still exist", count);
    }

    Ok(())
}
