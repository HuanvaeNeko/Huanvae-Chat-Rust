use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SubmitFriendRequest {
    pub user_id: String,
    pub target_user_id: String,
    pub reason: Option<String>,
    pub request_time: String,
}

#[derive(Debug, Serialize)]
pub struct SubmitFriendResponse {
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ApproveFriendRequest {
    pub user_id: String,
    pub applicant_user_id: String,
    pub approved_time: String,
    pub approved_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RejectFriendRequest {
    pub user_id: String,
    pub applicant_user_id: String,
    pub reject_reason: Option<String>,
}