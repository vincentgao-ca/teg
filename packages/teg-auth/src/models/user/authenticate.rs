// use chrono::prelude::*;
use juniper::{
    FieldResult,
};

use crate::ResultExt;
use super::User;
use super::jwt::validate_jwt;
// use crate::models::{ Invite };
use crate::{ Context };

impl User {
    pub async fn authenticate(
        context: &Context,
        auth_token: String,
        identity_public_key: String
    ) -> FieldResult<Option<User>> {
        let jwt_payload = validate_jwt(context, auth_token).await?;

        /*
        * Verify that either:
        * 1. the public key belongs to an invite
        * OR
        * 2. the user's token is authorized
        */
        let mut db = context.db().await?;

        let invite = sqlx::query!(
            "SELECT id FROM invites WHERE public_key=?",
            identity_public_key
        )
            .fetch_optional(&mut db)
            .await
            .chain_err(|| "Unable to load invite for authentication")?;

        if invite.is_none() {
            let user = sqlx::query!(
                "SELECT id FROM users WHERE firebase_uid=? AND is_authorized=True",
                jwt_payload.sub
            )
                .fetch_optional(&mut db)
                .await
                .chain_err(|| "Unable to load user for authentication")?;

            if user.is_none() {
                return Ok(None)
            }
        }

        eprintln!("JWT Payload: {:?}", jwt_payload);
        let now = Utc::now();

        /*
        * Upsert and return the user
        */
        let user = sqlx::query_as!(
            User,
            "
                INSERT INTO users (
                    firebase_uid,
                    email,
                    email_verified,
                    created_at,
                    last_logged_in_at
                )
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT (firebase_uid) DO UPDATE SET
                    email = ?,
                    email_verified = ?,
                    last_logged_in_at = ?
                RETURNING *
            ",
            jwt_payload.sub,
            jwt_payload.email,
            jwt_payload.email_verified,
            now.clone(),
            now.clone(),
            jwt_payload.email,
            jwt_payload.email_verified,
            now.clone()
        )
            .fetch_one(&mut db)
            .await
            .chain_err(|| "Unable to update user after authentication")?;

        eprintln!("User Authorized: {:?}", user);

        Ok(Some(user))
    }
}
