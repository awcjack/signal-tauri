//! Notification service

use crate::signal::messages::Message;
use notify_rust::{Notification, Timeout};

/// Send a message notification
pub fn notify_message(
    sender_name: &str,
    message_preview: &str,
    show_preview: bool,
    show_sender: bool,
) -> anyhow::Result<()> {
    let mut notification = Notification::new();

    notification
        .appname("Signal")
        .summary(if show_sender {
            sender_name
        } else {
            "New Message"
        })
        .timeout(Timeout::Milliseconds(5000));

    if show_preview {
        notification.body(message_preview);
    } else {
        notification.body("New message received");
    }

    // Set icon (platform-specific)
    #[cfg(target_os = "macos")]
    notification.subtitle("Signal");

    notification.show()?;

    Ok(())
}

/// Send a call notification
pub fn notify_call(
    caller_name: &str,
    is_video: bool,
) -> anyhow::Result<()> {
    let call_type = if is_video { "Video call" } else { "Voice call" };

    Notification::new()
        .appname("Signal")
        .summary(&format!("{} from {}", call_type, caller_name))
        .body("Tap to answer")
        .timeout(Timeout::Never)
        .show()?;

    Ok(())
}

/// Send a group notification
pub fn notify_group_event(
    group_name: &str,
    event: &str,
) -> anyhow::Result<()> {
    Notification::new()
        .appname("Signal")
        .summary(group_name)
        .body(event)
        .timeout(Timeout::Milliseconds(3000))
        .show()?;

    Ok(())
}

/// Clear all notifications for a conversation
pub fn clear_conversation_notifications(_conversation_id: &str) {
    // TODO: Implement platform-specific notification clearing
}

/// Update badge count (dock/taskbar)
pub fn update_badge_count(count: u32) {
    // TODO: Implement platform-specific badge updates
    tracing::debug!("Badge count: {}", count);
}
