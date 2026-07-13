use std::fmt;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{self, DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::group::models::{CreateGroupInput, Group, GroupMember, Role};

#[derive(Debug)]
pub enum GroupRepoError {
    DuplicateMember,
    Database(mongodb::error::Error),
}

impl fmt::Display for GroupRepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GroupRepoError::DuplicateMember => {
                write!(f, "user is already a member of this group")
            }
            GroupRepoError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for GroupRepoError {}

impl From<mongodb::error::Error> for GroupRepoError {
    fn from(err: mongodb::error::Error) -> Self {
        if is_duplicate_key(&err) {
            GroupRepoError::DuplicateMember
        } else {
            GroupRepoError::Database(err)
        }
    }
}

fn is_duplicate_key(err: &mongodb::error::Error) -> bool {
    use mongodb::error::{ErrorKind, WriteFailure};
    matches!(
        err.kind.as_ref(),
        ErrorKind::Write(WriteFailure::WriteError(e)) if e.code == 11000
    )
}

pub struct GroupRepository {
    groups: Collection<Group>,
    members: Collection<GroupMember>,
}

impl GroupRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            groups: db.collection("groups"),
            members: db.collection("group_members"),
        }
    }

    pub async fn create_group(&self, input: CreateGroupInput) -> Result<Group, GroupRepoError> {
        let group = Group {
            id: None,
            name: input.name,
            owner_id: input.owner_id,
            created_at: BsonDateTime::now(),
        };
        let result = self.groups.insert_one(&group).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .expect("insert_one always returns an ObjectId");
        Ok(Group {
            id: Some(id),
            ..group
        })
    }

    pub async fn find_group_by_id(&self, id: ObjectId) -> Result<Option<Group>, GroupRepoError> {
        Ok(self.groups.find_one(doc! { "_id": id }).await?)
    }

    pub async fn list_all_groups(&self) -> Result<Vec<Group>, GroupRepoError> {
        let cursor = self.groups.find(doc! {}).await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    pub async fn delete_group(&self, id: ObjectId) -> Result<bool, GroupRepoError> {
        let result = self.groups.delete_one(doc! { "_id": id }).await?;
        Ok(result.deleted_count > 0)
    }

    pub async fn rename_group(&self, id: ObjectId, name: String) -> Result<bool, GroupRepoError> {
        let result = self
            .groups
            .update_one(doc! { "_id": id }, doc! { "$set": { "name": name } })
            .await?;
        Ok(result.modified_count > 0)
    }

    pub async fn insert_member(
        &self,
        group_id: ObjectId,
        user_id: ObjectId,
        role: Role,
    ) -> Result<GroupMember, GroupRepoError> {
        let member = GroupMember {
            id: None,
            group_id,
            user_id,
            role,
            joined_at: BsonDateTime::now(),
        };
        let result = self.members.insert_one(&member).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .expect("insert_one always returns an ObjectId");
        Ok(GroupMember {
            id: Some(id),
            ..member
        })
    }

    pub async fn find_member(
        &self,
        group_id: ObjectId,
        user_id: ObjectId,
    ) -> Result<Option<GroupMember>, GroupRepoError> {
        Ok(self
            .members
            .find_one(doc! { "group_id": group_id, "user_id": user_id })
            .await?)
    }

    pub async fn list_members(&self, group_id: ObjectId) -> Result<Vec<GroupMember>, GroupRepoError> {
        let cursor = self.members.find(doc! { "group_id": group_id }).await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    pub async fn list_groups_for_user(&self, user_id: ObjectId) -> Result<Vec<Group>, GroupRepoError> {
        let cursor = self.members.find(doc! { "user_id": user_id }).await?;
        let memberships: Vec<GroupMember> = cursor.try_collect().await?;
        if memberships.is_empty() {
            return Ok(Vec::new());
        }
        let group_ids: Vec<ObjectId> = memberships.iter().map(|m| m.group_id).collect();
        let cursor = self
            .groups
            .find(doc! { "_id": { "$in": group_ids } })
            .await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    pub async fn count_group_admins(&self, group_id: ObjectId) -> Result<u64, GroupRepoError> {
        let role = bson::to_bson(&Role::GroupAdmin).expect("Role always serializes");
        Ok(self
            .members
            .count_documents(doc! { "group_id": group_id, "role": role })
            .await?)
    }

    pub async fn update_member_role(
        &self,
        group_id: ObjectId,
        user_id: ObjectId,
        role: Role,
    ) -> Result<bool, GroupRepoError> {
        let role = bson::to_bson(&role).expect("Role always serializes");
        let result = self
            .members
            .update_one(
                doc! { "group_id": group_id, "user_id": user_id },
                doc! { "$set": { "role": role } },
            )
            .await?;
        Ok(result.modified_count > 0)
    }

    pub async fn delete_member(
        &self,
        group_id: ObjectId,
        user_id: ObjectId,
    ) -> Result<bool, GroupRepoError> {
        let result = self
            .members
            .delete_one(doc! { "group_id": group_id, "user_id": user_id })
            .await?;
        Ok(result.deleted_count > 0)
    }

    pub async fn delete_members_by_group(&self, group_id: ObjectId) -> Result<u64, GroupRepoError> {
        let result = self
            .members
            .delete_many(doc! { "group_id": group_id })
            .await?;
        Ok(result.deleted_count)
    }
}
