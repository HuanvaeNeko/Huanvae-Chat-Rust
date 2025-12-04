use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SentRequestDto {
    pub request_id: String,
    pub sent_to_user_id: String,
    pub sent_message: Option<String>,
    pub sent_time: String,
}

#[derive(Debug, Serialize)]
pub struct PendingRequestDto {
    pub request_id: String,
    pub request_user_id: String,
    pub request_message: Option<String>,
    pub request_time: String,
}

#[derive(Debug, Serialize)]
pub struct FriendDto {
    pub friend_id: String,
    pub friend_nickname: Option<String>,
    pub friend_avatar_url: Option<String>,
    pub add_time: String,
    pub approve_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
}