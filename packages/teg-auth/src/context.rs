use juniper::{FieldResult, FieldError};
use std::sync::Arc;

use crate::models::User;
use async_std::sync::RwLock;
use crate::configuration::Config;

type SqlxError = sqlx::Error;

pub struct Context {
    pub pool: Arc<sqlx::SqlitePool>,
    pub current_user: Option<User>,
    pub identity_public_key: Option<String>,
    pub auth_pem_keys: Arc<RwLock<Vec<Vec<u8>>>>,
    pub machine_config: Arc<RwLock<Config>>,
}

// To make our context usable by Juniper, we have to implement a marker trait.
impl juniper::Context for Context {}

impl Context {
    pub async fn new(
        pool: Arc<sqlx::SqlitePool>,
        current_user_id: Option<i32>,
        identity_public_key: Option<String>,
        auth_pem_keys: Arc<RwLock<Vec<Vec<u8>>>>,
        machine_config: Arc<RwLock<Config>>,
    ) -> Result<Self, SqlxError> {
        let mut context = Self {
            pool,
            current_user: None,
            identity_public_key,
            auth_pem_keys,
            machine_config,
        };

        if let Some(current_user_id) = current_user_id {
            context.current_user  = sqlx::query_as!(
                User,
                "SELECT * FROM users WHERE id = $1",
                current_user_id
            )
                .fetch_optional(&mut context.db().await?)
                .await?;
        }

        Ok(context)
    }

    pub async fn db(
        &self
    ) -> sqlx::Result<sqlx::pool::PoolConnection<sqlx::SqliteConnection>> {
        self.pool.acquire().await
    }

    pub async fn tx(
        &self
    ) -> sqlx::Result<sqlx::Transaction<sqlx::pool::PoolConnection<sqlx::SqliteConnection>>> {
        self.pool.begin().await
    }

    pub fn is_admin(&self) -> bool {
        self.current_user
            .as_ref()
            .map(|user| user.is_admin)
            .unwrap_or(false)
    }

    pub fn authorize_admins_only(&self) -> FieldResult<()> {
        if self.is_admin() {
            Ok(())
        } else  {
            Err(FieldError::new(
                "Unauthorized",
                graphql_value!({ "internal_error": "Unauthorized" }),
            ))
        }
    }
}
