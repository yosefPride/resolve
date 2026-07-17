use mongodb::{Database, bson::oid::ObjectId};

use crate::user::{
    models::{CreateUserInput, User, UserResponse},
    repository::{UserRepoError, UserRepository},
};

pub struct UserService {
    repo: UserRepository,
}

impl UserService {
    pub fn new(db: &Database) -> Self {
        Self {
            repo: UserRepository::new(db),
        }
    }

    pub async fn create(&self, input: CreateUserInput) -> Result<UserResponse, UserRepoError> {
        self.repo.create(input).await.map(Into::into)
    }

    // Returns the full User (including password_hash) — intentional, needed by auth for login.
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserRepoError> {
        self.repo.find_by_email(email).await
    }

    // Returns the full User (including password_hash) — intentional, needed by
    // auth to verify the current password before an email change.
    pub async fn find_full_by_id(&self, id: ObjectId) -> Result<Option<User>, UserRepoError> {
        self.repo.find_by_id(id).await
    }

    pub async fn update_profile(
        &self,
        id: ObjectId,
        name: &str,
        email: &str,
    ) -> Result<Option<UserResponse>, UserRepoError> {
        self.repo
            .update_profile(id, name, email)
            .await
            .map(|opt| opt.map(Into::into))
    }

    pub async fn update_password_hash(
        &self,
        id: ObjectId,
        password_hash: &str,
    ) -> Result<bool, UserRepoError> {
        self.repo.update_password_hash(id, password_hash).await
    }

    pub async fn find_by_id(&self, id: ObjectId) -> Result<Option<UserResponse>, UserRepoError> {
        self.repo
            .find_by_id(id)
            .await
            .map(|opt| opt.map(Into::into))
    }

    pub async fn list_all(&self) -> Result<Vec<UserResponse>, UserRepoError> {
        self.repo
            .list_all(None)
            .await
            .map(|users| users.into_iter().map(Into::into).collect())
    }

    pub async fn delete(&self, id: ObjectId) -> Result<bool, UserRepoError> {
        self.repo.delete(id).await
    }
}
