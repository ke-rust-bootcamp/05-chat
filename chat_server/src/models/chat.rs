use crate::AppError;

use super::{Chat, ChatType, ChatUser};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateChat {
    pub name: Option<String>,
    pub members: Vec<i64>,
    pub public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateChat {
    pub name: Option<String>,
    pub members: Vec<i64>,
    pub public: bool,
}

#[allow(dead_code)]
impl Chat {
    pub async fn create(input: CreateChat, ws_id: u64, pool: &PgPool) -> Result<Self, AppError> {
        // nums of chat members can not less than 2
        let nums = input.members.len();
        if nums < 2 {
            return Err(AppError::CreateChatError(
                "nums of chat members can not less than 2".to_string(),
            ));
        }
        // if members num large than 8, chat must have a name
        if nums > 8 && input.name.is_none() {
            return Err(AppError::CreateChatError(
                "Group chat with more than 8 members must have a name".to_string(),
            ));
        }
        // check members exists
        let users = ChatUser::fetch_by_ids(&input.members, pool).await?;
        if users.len() != nums {
            return Err(AppError::CreateChatError(
                "Some members do not exist".to_string(),
            ));
        }

        let chat_type = match (&input.name, nums) {
            (None, 2) => ChatType::Single,
            (None, _) => ChatType::Group,
            (Some(_), _) => {
                if input.public {
                    ChatType::PublicChannel
                } else {
                    ChatType::PrivateChannel
                }
            }
        };

        let chat = sqlx::query_as(
            r#"
            INSERT INTO chats (ws_id, name, type, members)
            VALUES ($1, $2, $3, $4)
            RETURNING id, ws_id, name, type, members, created_at
            "#,
        )
        .bind(ws_id as i64)
        .bind(input.name)
        .bind(chat_type)
        .bind(&input.members)
        .fetch_one(pool)
        .await?;

        Ok(chat)
    }

    pub async fn fetch_all(ws_id: u64, pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let chats = sqlx::query_as(
            r#"
            SELECT id, ws_id, name, type, members, created_at
            FROM chats
            WHERE ws_id = $1
            "#,
        )
        .bind(ws_id as i64)
        .fetch_all(pool)
        .await?;

        Ok(chats)
    }

    pub async fn get_by_id(id: u64, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let chat = sqlx::query_as(
            r#"
            SELECT id, ws_id, name, type, members, created_at
            FROM chats
            WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(pool)
        .await?;

        Ok(chat)
    }

    pub async fn update_by_id(
        id: i64,
        ws_id: i64,
        input: UpdateChat,
        pool: &PgPool,
    ) -> Result<Self, AppError> {
        // 检查聊天是否存在
        let chat_update = Chat::get_by_id(id as _, pool).await?;
        if chat_update.is_none() {
            return Err(AppError::UpdateChatError("Chat does not exist".to_string()));
        };

        // 准备更新的字段
        let members = input.members;

        // 如果更新了成员，检查成员是否存在
        let users = ChatUser::fetch_by_ids(&members, pool).await?;
        if users.len() != members.len() {
            return Err(AppError::UpdateChatError(
                "Some members do not exist".to_string(),
            ));
        }
        let chat_type = match (&input.name, users.len()) {
            (None, 2) => ChatType::Single,
            (None, _) => ChatType::Group,
            (Some(_), _) => {
                if input.public {
                    ChatType::PublicChannel
                } else {
                    ChatType::PrivateChannel
                }
            }
        };

        // 构建查询，根据传入的参数更新字段
        let updated_chat = sqlx::query_as::<_, Chat>(
            r#"
        UPDATE chats
        SET
            name = COALESCE($1, name),
            type = COALESCE($2, type),
            members = COALESCE($3, members),
            ws_id = COALESCE($4, ws_id)
        WHERE id = $5
        RETURNING id, ws_id, name, type, members, created_at
        "#,
        )
        .bind(input.name)
        .bind(chat_type)
        .bind(members)
        .bind(ws_id)
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(updated_chat)
    }
}

#[cfg(test)]
impl CreateChat {
    pub fn new(name: &str, members: &[i64], public: bool) -> Self {
        let name = if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
        Self {
            name,
            members: members.to_vec(),
            public,
        }
    }
}

#[cfg(test)]
impl UpdateChat {
    pub fn new(name: &str, members: &[i64], public: bool) -> Self {
        // 如果 name 是一个空字符串，则不设置 name 字段
        let name = if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
        Self {
            name,
            members: members.to_vec(),
            public,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::get_test_pool;

    use super::*;

    #[tokio::test]
    async fn create_single_chat_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let input = CreateChat::new("", &[1, 2], false);
        let chat = Chat::create(input, 1, &pool)
            .await
            .expect("create chat failed");
        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 2);
        assert_eq!(chat.r#type, ChatType::Single);
    }

    #[tokio::test]
    async fn update_single_chat_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let input = CreateChat::new("", &[1, 2], false);
        let chat = Chat::create(input, 1, &pool)
            .await
            .expect("create chat failed");
        let input = UpdateChat::new("fps", &[1, 2, 3], true);
        let chat = Chat::update_by_id(chat.id, 1, input, &pool)
            .await
            .expect("update chat failed");
        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 3);
        assert_eq!(chat.r#type, ChatType::PublicChannel);
    }
}
