//! ICE 服务器配置 API

use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde::Deserialize;

use crate::auth::middleware::AuthContext;
use crate::common::{ApiResponse, AppError};
use crate::turn::models::protocol::IceServersResponse;

use super::TurnState;

/// 请求参数
#[derive(Debug, Deserialize)]
pub struct IceServersParams {
    /// 客户端区域（可选）
    pub region: Option<String>,
}

/// 获取 ICE 服务器配置
///
/// GET /api/webrtc/ice-servers
pub async fn get_ice_servers(
    State(turn_state): State<TurnState>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<IceServersParams>,
) -> Result<Json<ApiResponse<IceServersResponse>>, AppError> {
    // 检查 TURN 是否启用
    if !turn_state.enabled {
        return Err(AppError::BadRequest("TURN 服务未启用".to_string()));
    }

    // 选择最优节点
    let selected_nodes = turn_state
        .load_balancer
        .select_nodes(params.region.as_deref(), 3);

    // 如果没有可用节点，仍然返回 STUN 配置
    if selected_nodes.is_empty() {
        tracing::warn!("没有可用的 TURN 节点");
    }

    // 生成凭证
    let response = turn_state
        .credential_service
        .generate_ice_servers(&auth.user_id, selected_nodes)
        .await;

    Ok(Json(ApiResponse::success(response)))
}
