//! UI Views - different screens/pages of the application

pub mod chat_list;
pub mod chat_view;
pub mod link_device;
pub mod main_view;
pub mod settings;

/// Current view state
#[derive(Debug, Clone, PartialEq)]
pub enum ViewState {
    /// Device linking screen (shown when no account exists)
    LinkDevice,
    /// Main chat list view
    ChatList,
    /// Settings view
    Settings,
}

impl Default for ViewState {
    fn default() -> Self {
        Self::LinkDevice
    }
}
