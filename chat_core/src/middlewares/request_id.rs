use super::REQUEST_ID_HEADER;
use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use tracing::warn;

pub async fn set_request_id(mut req: Request, next: Next) -> Response {
    let id = match req.headers().get(REQUEST_ID_HEADER) {
        Some(v) => Some(v.clone()),
        None => {
            let id = uuid::Uuid::now_v7().to_string();
            match HeaderValue::from_str(&id) {
                Ok(v) => {
                    req.headers_mut().insert(REQUEST_ID_HEADER, v.clone());
                    Some(v)
                }
                Err(e) => {
                    warn!("parse generated request id failed: {}", e);
                    None
                }
            }
        }
    };

    let mut res = next.run(req).await;
    // 使用模式匹配来检查id是否为Some，如果不是（即之前生成或获取ID失败），则直接返回响应
    let Some(id) = id else {
        return res;
    };
    res.headers_mut().insert(REQUEST_ID_HEADER, id);
    res
}
