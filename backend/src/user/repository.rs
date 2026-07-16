use std::fmt;

use futures::TryStreamExt;
use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::user::models::{CreateUserInput, User};

#[derive(Debug)]
pub enum UserRepoError {
    DuplicateEmail,
    Database(mongodb::error::Error),
}

impl fmt::Display for UserRepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserRepoError::DuplicateEmail => write!(f, "email already in use"),
            UserRepoError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for UserRepoError {}

impl From<mongodb::error::Error> for UserRepoError {
    fn from(err: mongodb::error::Error) -> Self {
        if is_duplicate_key(&err) {
            UserRepoError::DuplicateEmail
        } else {
            UserRepoError::Database(err)
        }
    }
}

// Duplicate-key surfaces as a WriteError on insert_one but as a CommandError
// on find_one_and_update (findAndModify is a command, not a plain write).
fn is_duplicate_key(err: &mongodb::error::Error) -> bool {
    use mongodb::error::{ErrorKind, WriteFailure};
    match err.kind.as_ref() {
        ErrorKind::Write(WriteFailure::WriteError(e)) => e.code == 11000,
        ErrorKind::Command(e) => e.code == 11000,
        _ => false,
    }
}

pub struct UserRepository {
    collection: Collection<User>,
}

impl UserRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection("users"),
        }
    }

    pub async fn create(&self, input: CreateUserInput) -> Result<User, UserRepoError> {
        let user = User {
            id: None,
            email: input.email,
            password_hash: input.password_hash,
            name: input.name,
            global_role: None,
            created_at: BsonDateTime::now(),
        };
        let result = self.collection.insert_one(&user).await?;
        let id = result
            .inserted_id
            .as_object_id()
            .expect("insert_one always returns an ObjectId");
        Ok(User {
            id: Some(id),
            ..user
        })
    }

    pub async fn update_profile(
        &self,
        id: ObjectId,
        name: &str,
        email: &str,
    ) -> Result<Option<User>, UserRepoError> {
        Ok(self
            .collection
            .find_one_and_update(
                doc! { "_id": id },
                doc! { "$set": { "name": name, "email": email } },
            )
            .return_document(mongodb::options::ReturnDocument::After)
            .await?)
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserRepoError> {
        Ok(self.collection.find_one(doc! { "email": email }).await?)
    }

    pub async fn find_by_id(&self, id: ObjectId) -> Result<Option<User>, UserRepoError> {
        Ok(self.collection.find_one(doc! { "_id": id }).await?)
    }

    pub async fn list_all(&self) -> Result<Vec<User>, UserRepoError> {
        let cursor = self.collection.find(doc! {}).await?;
        cursor.try_collect().await.map_err(Into::into)
    }

    pub async fn delete(&self, id: ObjectId) -> Result<bool, UserRepoError> {
        let result = self.collection.delete_one(doc! { "_id": id }).await?;
        Ok(result.deleted_count > 0)
    }
}
