use chrono::{DateTime, Utc};
use mongodb::bson::{DateTime as BsonDateTime, oid::ObjectId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Contributor,
    GroupAdmin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub owner_id: ObjectId,
    pub created_at: BsonDateTime,
}

pub struct CreateGroupInput {
    pub name: String,
    pub owner_id: ObjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub group_id: ObjectId,
    pub user_id: ObjectId,
    pub role: Role,
    pub joined_at: BsonDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupResponse {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
}

impl From<Group> for GroupResponse {
    fn from(group: Group) -> Self {
        GroupResponse {
            id: group.id.map(|id| id.to_hex()).unwrap_or_default(),
            name: group.name,
            owner_id: group.owner_id.to_hex(),
            created_at: DateTime::from_timestamp_millis(group.created_at.timestamp_millis())
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberResponse {
    pub id: String,
    pub user_id: String,
    pub role: Role,
    pub joined_at: DateTime<Utc>,
}

impl From<GroupMember> for MemberResponse {
    fn from(member: GroupMember) -> Self {
        MemberResponse {
            id: member.id.map(|id| id.to_hex()).unwrap_or_default(),
            user_id: member.user_id.to_hex(),
            role: member.role,
            joined_at: DateTime::from_timestamp_millis(member.joined_at.timestamp_millis())
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: String,
    pub role: Role,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub role: Role,
}
