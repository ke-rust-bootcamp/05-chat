use crate::{
    models::{CreateUser, SigninUser},
    AppError, AppState, ErrorOutput, User,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthOutput {
    token: String,
}

pub(crate) async fn signup_handler(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::create(&input, &state.pool).await?;
    let token = state.ek.sign(user)?;
    let body = Json(AuthOutput { token });
    Ok((StatusCode::CREATED, body))
}

pub(crate) async fn signin_handler(
    State(state): State<AppState>,
    Json(input): Json<SigninUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::verify(&input, &state.pool).await?;

    match user {
        Some(user) => {
            let token = state.ek.sign(user)?;
            Ok((StatusCode::OK, Json(AuthOutput { token })).into_response())
        }
        None => {
            let body = Json(ErrorOutput::new("Invalid email or password"));
            Ok((StatusCode::FORBIDDEN, body).into_response())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppConfig;
    use anyhow::Result;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn signup_should_work() -> Result<()> {
        let config = AppConfig::load()?;
        let (_tdb, state) = AppState::new_for_test(config).await?;
        let input = CreateUser::new("Ke Lei", "klei@foxmail.com", "postgres");
        let ret = signup_handler(State(state), Json(input))
            .await?
            .into_response();
        assert_eq!(ret.status(), StatusCode::CREATED);
        let body = ret.into_body().collect().await?.to_bytes();
        let ret: AuthOutput = serde_json::from_slice(&body)?;
        assert_ne!(ret.token, "");
        Ok(())
    }

    #[tokio::test]
    async fn signin_should_work() -> Result<()> {
        let config = AppConfig::load()?;
        let (_tdb, state) = AppState::new_for_test(config).await?;
        let email = "klei@foxmail.com";
        let password = "postgres";
        let name = "ke Lei";
        let user = CreateUser::new(name, email, password);
        User::create(&user, &state.pool).await?;
        let ret = signin_handler(State(state), Json(SigninUser::new(email, password)))
            .await
            .into_response();
        let status_code1 = ret.status();

        assert_eq!(status_code1, StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn signup_duplicate_user_should_409() -> Result<()> {
        let config = AppConfig::load()?;
        let (_tdb, state) = AppState::new_for_test(config).await?;
        let email = "klei@foxmail.com";
        let password = "postgres";
        let name = "ke Lei";
        let user = CreateUser::new(name, email, password);
        signup_handler(State(state.clone()), Json(user.clone())).await?;
        // 以下不使用 ? 操作符，防止错误发生后直接返回导致测试失败
        let ret = signup_handler(State(state), Json(user))
            .await
            .into_response();
        assert_eq!(ret.status(), StatusCode::CONFLICT);
        let body = ret.into_body().collect().await?.to_bytes();
        let ret_message: ErrorOutput = serde_json::from_slice(&body)?;
        assert_eq!(ret_message.error, "email already exists: klei@foxmail.com");
        Ok(())
    }

    #[tokio::test]
    async fn signin_with_non_exist_user_should_403() -> Result<()> {
        let config = AppConfig::load()?;
        let (_tdb, state) = AppState::new_for_test(config).await?;
        let email = "klei@foxmail.com";
        let password = "postgres";
        // let name = "ke Lei";
        // let user = CreateUser::new(name, email, password);
        // User::create(&user, &state.pool).await?;
        let ret = signin_handler(State(state), Json(SigninUser::new(email, password)))
            .await
            .into_response();
        assert_eq!(ret.status(), StatusCode::FORBIDDEN);
        let body = ret.into_body().collect().await?.to_bytes();
        let ret_message: ErrorOutput = serde_json::from_slice(&body)?;
        assert_eq!(ret_message.error, "Invalid email or password");
        Ok(())
    }
}
