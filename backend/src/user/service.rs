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

    pub async fn find_by_id(&self, id: ObjectId) -> Result<Option<UserResponse>, UserRepoError> {
        self.repo.find_by_id(id).await.map(|opt| opt.map(Into::into))
    }

    // Returns the full User (including token_version) — needed by the auth
    // extractor to check a token's version against the user's current one.
    pub async fn find_raw_by_id(&self, id: ObjectId) -> Result<Option<User>, UserRepoError> {
        self.repo.find_by_id(id).await
    }

    pub async fn increment_token_version(&self, id: ObjectId) -> Result<(), UserRepoError> {
        self.repo.increment_token_version(id).await
    }

    pub async fn list_all(&self) -> Result<Vec<UserResponse>, UserRepoError> {
        self.repo
            .list_all()
            .await
            .map(|users| users.into_iter().map(Into::into).collect())
    }

    pub async fn delete(&self, id: ObjectId) -> Result<bool, UserRepoError> {
        self.repo.delete(id).await
    }
}
