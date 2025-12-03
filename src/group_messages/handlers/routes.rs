//! 群消息路由配置

use axum::{
    routing::{delete, get, post},
    Router,
};
use crate::auth::middleware::{auth_guard, AuthState};
use super::state::GroupMessagesState;
use super::{
    send_message::send_message,
    get_messages::get_messages,
    delete_message::delete_message,
    recall_message::recall_message,
};

/// 创建群消息路由
pub fn create_group_messages_routes(state: GroupMessagesState, auth_state: AuthState) -> Router {
    Router::new()
        .route("/", post(send_message))
        .route("/", get(get_messages))
        .route("/delete", delete(delete_message))
        .route("/recall", post(recall_message))
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(auth_state, auth_guard))
}

