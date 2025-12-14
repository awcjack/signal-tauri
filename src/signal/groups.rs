//! Group management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Group access control
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AccessControl {
    /// Any member can modify
    Any,
    /// Only admins can modify
    Administrator,
    /// Not supported (e.g., can't invite anyone)
    Unsatisfiable,
}

impl Default for AccessControl {
    fn default() -> Self {
        Self::Any
    }
}

/// Group member role
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MemberRole {
    Default,
    Administrator,
}

impl Default for MemberRole {
    fn default() -> Self {
        Self::Default
    }
}

/// A group member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    /// Member's UUID
    pub uuid: String,

    /// Member's role
    pub role: MemberRole,

    /// When they joined
    pub joined_at: DateTime<Utc>,
}

/// A pending group member (invited but not joined)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingMember {
    /// Member's UUID
    pub uuid: String,

    /// Who invited them
    pub added_by: String,

    /// When they were invited
    pub invited_at: DateTime<Utc>,
}

/// A Signal group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    /// Group ID (V2 groups use a base64-encoded master key derivative)
    pub id: String,

    /// Group master key (for V2 groups)
    pub master_key: Option<Vec<u8>>,

    /// Group name
    pub name: String,

    /// Group description
    pub description: Option<String>,

    /// Avatar path
    pub avatar_path: Option<String>,

    /// Group members
    pub members: Vec<GroupMember>,

    /// Pending members (invited but not joined)
    pub pending_members: Vec<PendingMember>,

    /// Who can edit group attributes
    pub attributes_access: AccessControl,

    /// Who can add new members
    pub members_access: AccessControl,

    /// Whether link sharing is enabled
    pub link_enabled: bool,

    /// Group invite link
    pub invite_link: Option<String>,

    /// Disappearing messages timer (0 = disabled)
    pub disappearing_messages_timer: u32,

    /// Whether the group is blocked
    pub blocked: bool,

    /// Whether we've left the group
    pub left: bool,

    /// Revision number (for conflict resolution)
    pub revision: u32,

    /// When the group was created
    pub created_at: DateTime<Utc>,

    /// Last group update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Group {
    /// Create a new group
    pub fn new(id: &str, name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            master_key: None,
            name: name.to_string(),
            description: None,
            avatar_path: None,
            members: Vec::new(),
            pending_members: Vec::new(),
            attributes_access: AccessControl::Any,
            members_access: AccessControl::Any,
            link_enabled: false,
            invite_link: None,
            disappearing_messages_timer: 0,
            blocked: false,
            left: false,
            revision: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Check if a user is a member
    pub fn is_member(&self, uuid: &str) -> bool {
        self.members.iter().any(|m| m.uuid == uuid)
    }

    /// Check if a user is an admin
    pub fn is_admin(&self, uuid: &str) -> bool {
        self.members
            .iter()
            .any(|m| m.uuid == uuid && m.role == MemberRole::Administrator)
    }

    /// Get initials for avatar
    pub fn initials(&self) -> String {
        self.name
            .split_whitespace()
            .take(2)
            .map(|word| word.chars().next().unwrap_or('?'))
            .collect::<String>()
            .to_uppercase()
    }

    /// Add a member
    pub fn add_member(&mut self, uuid: &str, role: MemberRole) {
        if !self.is_member(uuid) {
            self.members.push(GroupMember {
                uuid: uuid.to_string(),
                role,
                joined_at: Utc::now(),
            });
            self.updated_at = Utc::now();
            self.revision += 1;
        }
    }

    /// Remove a member
    pub fn remove_member(&mut self, uuid: &str) {
        self.members.retain(|m| m.uuid != uuid);
        self.updated_at = Utc::now();
        self.revision += 1;
    }

    /// Promote a member to admin
    pub fn promote_to_admin(&mut self, uuid: &str) {
        if let Some(member) = self.members.iter_mut().find(|m| m.uuid == uuid) {
            member.role = MemberRole::Administrator;
            self.updated_at = Utc::now();
            self.revision += 1;
        }
    }

    /// Demote an admin to regular member
    pub fn demote_admin(&mut self, uuid: &str) {
        if let Some(member) = self.members.iter_mut().find(|m| m.uuid == uuid) {
            member.role = MemberRole::Default;
            self.updated_at = Utc::now();
            self.revision += 1;
        }
    }
}

/// Group repository for storage operations
pub struct GroupRepository {
    // TODO: Add storage backend
}

impl GroupRepository {
    /// Create a new group repository
    pub fn new() -> Self {
        Self {}
    }

    /// Get a group by ID
    pub async fn get(&self, id: &str) -> Option<Group> {
        // TODO: Implement storage lookup
        None
    }

    /// Save a group
    pub async fn save(&self, group: &Group) -> anyhow::Result<()> {
        // TODO: Implement storage save
        Ok(())
    }

    /// Get all groups
    pub async fn list(&self) -> Vec<Group> {
        // TODO: Implement storage list
        Vec::new()
    }

    /// Get active groups (not left, not blocked)
    pub async fn list_active(&self) -> Vec<Group> {
        // TODO: Implement storage list with filter
        Vec::new()
    }

    /// Delete a group
    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        // TODO: Implement delete
        Ok(())
    }
}

impl Default for GroupRepository {
    fn default() -> Self {
        Self::new()
    }
}
