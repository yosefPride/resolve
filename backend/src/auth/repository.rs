use mongodb::{
    Collection, Database,
    bson::{DateTime as BsonDateTime, doc, oid::ObjectId},
};

use crate::auth::models::RefreshTokenDoc;

pub struct AuthRepository {
    collection: Collection<RefreshTokenDoc>,
}

impl AuthRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection("refresh_tokens"),
        }
    }

    pub async fn insert(
        &self,
        user_id: ObjectId,
        token_hash: String,
        expires_at: BsonDateTime,
    ) -> Result<(), mongodb::error::Error> {
        let doc = RefreshTokenDoc {
            id: None,
            user_id,
            token_hash,
            created_at: BsonDateTime::now(),
            expires_at,
            revoked_at: None,
        };
        self.collection.insert_one(&doc).await?;
        Ok(())
    }

    // Matches only a token that hasn't been revoked and hasn't expired — a
    // stolen-then-replayed token (already revoked by the legitimate rotation)
    // or a stale one simply won't be found, no separate reuse handling needed.
    pub async fn find_active_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshTokenDoc>, mongodb::error::Error> {
        self.collection
            .find_one(doc! {
                "token_hash": token_hash,
                "revoked_at": null,
                "expires_at": { "$gt": BsonDateTime::now() },
            })
            .await
    }

    pub async fn revoke_by_id(&self, id: ObjectId) -> Result<(), mongodb::error::Error> {
        self.collection
            .update_one(
                doc! { "_id": id },
                doc! { "$set": { "revoked_at": BsonDateTime::now() } },
            )
            .await?;
        Ok(())
    }

    // Revokes every outstanding refresh token for a user except the one whose
    // hash is given — password change logs out all other devices while the
    // session that made the change stays alive. `except_hash: None` (no
    // refresh cookie on the request) revokes them all.
    pub async fn revoke_all_for_user_except(
        &self,
        user_id: ObjectId,
        except_hash: Option<&str>,
    ) -> Result<(), mongodb::error::Error> {
        let mut filter = doc! { "user_id": user_id, "revoked_at": null };
        if let Some(hash) = except_hash {
            filter.insert("token_hash", doc! { "$ne": hash });
        }
        self.collection
            .update_many(
                filter,
                doc! { "$set": { "revoked_at": BsonDateTime::now() } },
            )
            .await?;
        Ok(())
    }

    pub async fn revoke_by_hash(&self, token_hash: &str) -> Result<(), mongodb::error::Error> {
        self.collection
            .update_one(
                doc! { "token_hash": token_hash },
                doc! { "$set": { "revoked_at": BsonDateTime::now() } },
            )
            .await?;
        Ok(())
    }
}
