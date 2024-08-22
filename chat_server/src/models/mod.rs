mod user;

use serde::{Deserialize, Serialize};
use sqlx::{
    types::chrono::{DateTime, Utc},
    FromRow,
};

pub use user::{CreateUser, SigninUser};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: i64,
    pub fullname: String,
    pub email: String,
    #[sqlx(default)]
    #[serde(skip)]
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}
