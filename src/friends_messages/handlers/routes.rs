use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};

use crate::auth::middleware::{auth_guard, AuthState};

use super::{
    delete_message::delete_message_handler, get_messages::get_messages_handler,
    recall_message::recall_message_handler, send_message::send_message_handler, state::MessagesState,
};

/// 创建消息路由
pub fn create_messages_routes(state: MessagesState, auth_state: AuthState) -> Router {
    Router::new()
        .route("/", post(send_message_handler))        // 发送消息
        .route("/", get(get_messages_handler))         // 获取消息列表
        .route("/delete", delete(delete_message_handler))  // 删除消息
        .route("/recall", post(recall_message_handler))    // 撤回消息
        .with_state(state)
        .layer(middleware::from_fn_with_state(auth_state, auth_guard))
}

