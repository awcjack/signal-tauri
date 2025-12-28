//! UI Views - different screens/pages of the application

pub mod chat_list;
pub mod chat_view;
pub mod encryption_setup;
pub mod link_device;
pub mod main_view;
pub mod settings;
pub mod unlock_database;

#[derive(Debug, Clone, PartialEq)]
pub enum ViewState {
    EncryptionSetup,
    LinkDevice,
    UnlockDatabase,
    ChatList,
    Settings,
}

impl Default for ViewState {
    fn default() -> Self {
        Self::EncryptionSetup
    }
}
