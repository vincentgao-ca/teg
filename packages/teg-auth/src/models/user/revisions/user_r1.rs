use chrono::prelude::*;
use juniper::{
    ID,
};
use serde::{Deserialize, Serialize};

// use super::{ User, UserR2 };

#[derive(Debug, Serialize, Deserialize)]
pub struct UserR1 {
    pub id: ID,
    pub email: Option<String>,
    pub email_verified: bool,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub last_logged_in_at: Option<DateTime<Utc>>,

    pub firebase_uid: String,
    pub is_authorized: bool,
}

// impl From<UserR1> for User {
//     fn from(r1: UserR1) -> Self {
//         UserR2 {
//         }.into()
//     }
// }
