use crate::auth::errors::AuthError;
use crate::auth::middleware::AuthContext;
use crate::friends::models::{ApproveFriendRequest, RejectFriendRequest, SubmitFriendRequest, SubmitFriendResponse, RemoveFriendRequest};
use crate::friends::services::{append_record, parse_records, serialize_records, set_status};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct FriendsState {
    pub db: PgPool,
}

impl FriendsState {
    pub fn new(db: PgPool) -> Self { Self { db } }
}

fn ensure_user_id_matches_token(req_user_id: &str, auth: &AuthContext) -> Result<(), AuthError> {
    if req_user_id != auth.user_id {
        return Err(AuthError::Unauthorized);
    }
    Ok(())
}

pub async fn submit_request(
    state: &FriendsState,
    auth: &AuthContext,
    body: SubmitFriendRequest,
) -> Result<SubmitFriendResponse, AuthError> {
    ensure_user_id_matches_token(&body.user_id, auth)?;

    let (mut applicant_sent,): (String,) = sqlx::query_as(
        r#"SELECT "user-sent-friend-requests" FROM "users" WHERE "user-id" = $1 FOR UPDATE"#,
    )
    .bind(&body.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    let (mut target_pending,): (String,) = sqlx::query_as(
        r#"SELECT "user-pending-friend-requests" FROM "users" WHERE "user-id" = $1 FOR UPDATE"#,
    )
    .bind(&body.target_user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    
    let mut sent_recs = parse_records(&applicant_sent);
    let mut pending_recs = parse_records(&target_pending);

    // 检查是否存在对向开放请求（自动互通过）
    let target_sent_text: (String,) = sqlx::query_as(
        r#"SELECT "user-sent-friend-requests" FROM "users" WHERE "user-id" = $1 FOR UPDATE"#,
    )
    .bind(&body.target_user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;
    let target_sent_recs = parse_records(&target_sent_text.0);

    let applicant_pending_text: (String,) = sqlx::query_as(
        r#"SELECT "user-pending-friend-requests" FROM "users" WHERE "user-id" = $1 FOR UPDATE"#,
    )
    .bind(&body.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;
    let mut applicant_pending_recs = parse_records(&applicant_pending_text.0);

    let exists_reverse_open = target_sent_recs.iter().any(|r| {
        r.get("sent-to-user-id")
            .map(|v| v == &body.user_id)
            .unwrap_or(false)
            && r.get("status").map(|s| s == "open").unwrap_or(true)
    });

    
    let request_id = Uuid::now_v7().to_string();
    let mut sent_new = std::collections::HashMap::new();
    sent_new.insert("request-id".into(), request_id.clone());
    sent_new.insert("sent-to-user-id".into(), body.target_user_id.clone());
    if let Some(reason) = body.reason.clone() { sent_new.insert("sent-message".into(), reason); }
    sent_new.insert("sent-time".into(), body.request_time.clone());
    append_record(&mut sent_recs, sent_new);

    let mut pending_new = std::collections::HashMap::new();
    pending_new.insert("request-id".into(), request_id.clone());
    pending_new.insert("request-user-id".into(), body.user_id.clone());
    if let Some(reason) = body.reason { pending_new.insert("request-message".into(), reason); }
    pending_new.insert("request-time".into(), body.request_time);
    append_record(&mut pending_recs, pending_new);

    
    applicant_sent = serialize_records(&sent_recs);
    target_pending = serialize_records(&pending_recs);

    sqlx::query(r#"UPDATE "users" SET "user-sent-friend-requests"=$1 WHERE "user-id"=$2"#)
        .bind(&applicant_sent)
        .bind(&body.user_id)
        .execute(&state.db)
        .await
        .map_err(|_| AuthError::InternalServerError)?;

    sqlx::query(r#"UPDATE "users" SET "user-pending-friend-requests"=$1 WHERE "user-id"=$2"#)
        .bind(&target_pending)
        .bind(&body.target_user_id)
        .execute(&state.db)
        .await
        .map_err(|_| AuthError::InternalServerError)?;

    
    if exists_reverse_open {
        auto_approve(state, &auth.user_id, &body.target_user_id, None).await?;
        set_status(
            &mut applicant_pending_recs,
            &|r| r.get("request-user-id").map(|v| v == &body.target_user_id).unwrap_or(false),
            "approved",
        );
        let applicant_pending_text = serialize_records(&applicant_pending_recs);
        sqlx::query(r#"UPDATE "users" SET "user-pending-friend-requests"=$1 WHERE "user-id"=$2"#)
            .bind(&applicant_pending_text)
            .bind(&auth.user_id)
            .execute(&state.db)
            .await
            .map_err(|_| AuthError::InternalServerError)?;
    }
    Ok(SubmitFriendResponse { request_id })
}

pub async fn approve_request(
    state: &FriendsState,
    auth: &AuthContext,
    body: ApproveFriendRequest,
) -> Result<(), AuthError> {
    ensure_user_id_matches_token(&body.user_id, auth)?;

    manual_approve(state, &auth.user_id, &body.applicant_user_id, body.approved_reason).await?;
    Ok(())
}

pub async fn reject_request(
    state: &FriendsState,
    auth: &AuthContext,
    body: RejectFriendRequest,
) -> Result<(), AuthError> {
    ensure_user_id_matches_token(&body.user_id, auth)?;

    let (mut applicant_sent,): (String,) = sqlx::query_as(
        r#"SELECT "user-sent-friend-requests" FROM "users" WHERE "user-id" = $1 FOR UPDATE"#,
    )
    .bind(&body.applicant_user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    let (mut approver_pending,): (String,) = sqlx::query_as(
        r#"SELECT "user-pending-friend-requests" FROM "users" WHERE "user-id" = $1 FOR UPDATE"#,
    )
    .bind(&auth.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    let mut sent_recs = parse_records(&applicant_sent);
    let mut pending_recs = parse_records(&approver_pending);

    set_status(
        &mut sent_recs,
        &|r| r.get("sent-to-user-id").map(|v| v == &auth.user_id).unwrap_or(false),
        "rejected",
    );
    set_status(
        &mut pending_recs,
        &|r| r.get("request-user-id").map(|v| v == &body.applicant_user_id).unwrap_or(false),
        "rejected",
    );

    if let Some(reason) = body.reject_reason {
        for r in pending_recs.iter_mut() {
            if r.get("request-user-id").map(|v| v == &body.applicant_user_id).unwrap_or(false) {
                r.insert("reject-reason".into(), reason.clone());
            }
        }
    }

    applicant_sent = serialize_records(&sent_recs);
    approver_pending = serialize_records(&pending_recs);

    sqlx::query(r#"UPDATE "users" SET "user-sent-friend-requests"=$1 WHERE "user-id"=$2"#)
        .bind(&applicant_sent)
        .bind(&body.applicant_user_id)
        .execute(&state.db)
        .await
        .map_err(|_| AuthError::InternalServerError)?;

    sqlx::query(r#"UPDATE "users" SET "user-pending-friend-requests"=$1 WHERE "user-id"=$2"#)
        .bind(&approver_pending)
        .bind(&auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|_| AuthError::InternalServerError)?;
    Ok(())
}

pub async fn remove_friend(
    state: &FriendsState,
    auth: &AuthContext,
    body: RemoveFriendRequest,
) -> Result<(), AuthError> {
    ensure_user_id_matches_token(&body.user_id, auth)?;

    let (mut owner_owned,): (String,) = sqlx::query_as(
        r#"SELECT "user-owned-friends" FROM "users" WHERE "user-id" = $1 FOR UPDATE"#,
    )
    .bind(&body.user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    let (mut friend_owned,): (String,) = sqlx::query_as(
        r#"SELECT "user-owned-friends" FROM "users" WHERE "user-id" = $1 FOR UPDATE"#,
    )
    .bind(&body.friend_user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    let mut owner_recs = parse_records(&owner_owned);
    let mut friend_recs = parse_records(&friend_owned);

    for r in owner_recs.iter_mut() {
        if r.get("friend-id").map(|v| v == &body.friend_user_id).unwrap_or(false) {
            r.insert("status".into(), "ended".into());
            r.insert("remove-time".into(), body.remove_time.clone());
            if let Some(reason) = body.remove_reason.clone() {
                r.insert("remove-reason".into(), reason);
            }
        }
    }

    for r in friend_recs.iter_mut() {
        if r.get("friend-id").map(|v| v == &body.user_id).unwrap_or(false) {
            r.insert("status".into(), "ended".into());
            r.insert("remove-time".into(), body.remove_time.clone());
            if let Some(reason) = body.remove_reason.clone() {
                r.insert("remove-reason".into(), reason);
            }
        }
    }

    owner_owned = serialize_records(&owner_recs);
    friend_owned = serialize_records(&friend_recs);

    sqlx::query(r#"UPDATE "users" SET "user-owned-friends"=$1 WHERE "user-id"=$2"#)
        .bind(&owner_owned)
        .bind(&body.user_id)
        .execute(&state.db)
        .await
        .map_err(|_| AuthError::InternalServerError)?;

    sqlx::query(r#"UPDATE "users" SET "user-owned-friends"=$1 WHERE "user-id"=$2"#)
        .bind(&friend_owned)
        .bind(&body.friend_user_id)
        .execute(&state.db)
        .await
        .map_err(|_| AuthError::InternalServerError)?;

    Ok(())
}

async fn manual_approve(
    state: &FriendsState,
    approver_user_id: &str,
    applicant_user_id: &str,
    approved_reason: Option<String>,
) -> Result<(), AuthError> {
    
    let (mut applicant_sent,): (String,) = sqlx::query_as(
        r#"SELECT "user-sent-friend-requests" FROM "users" WHERE "user-id" = $1 FOR UPDATE"#,
    )
    .bind(&applicant_user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    let (mut approver_pending,): (String,) = sqlx::query_as(
        r#"SELECT "user-pending-friend-requests" FROM "users" WHERE "user-id" = $1 FOR UPDATE"#,
    )
    .bind(&approver_user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    let (mut applicant_owned,): (String,) = sqlx::query_as(
        r#"SELECT "user-owned-friends" FROM "users" WHERE "user-id" = $1 FOR UPDATE"#,
    )
    .bind(&applicant_user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    let (mut approver_owned,): (String,) = sqlx::query_as(
        r#"SELECT "user-owned-friends" FROM "users" WHERE "user-id" = $1 FOR UPDATE"#,
    )
    .bind(&approver_user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AuthError::InvalidToken)?;

    let mut sent_recs = parse_records(&applicant_sent);
    let mut pending_recs = parse_records(&approver_pending);
    let mut applicant_friends = parse_records(&applicant_owned);
    let mut approver_friends = parse_records(&approver_owned);

    
    set_status(
        &mut sent_recs,
        &|r| r.get("sent-to-user-id").map(|v| v == approver_user_id).unwrap_or(false),
        "approved",
    );
    set_status(
        &mut pending_recs,
        &|r| r.get("request-user-id").map(|v| v == applicant_user_id).unwrap_or(false),
        "approved",
    );

    
    let mut applicant_friend = std::collections::HashMap::new();
    applicant_friend.insert("friend-id".into(), approver_user_id.to_string());
    applicant_friend.insert("add-time".into(), chrono::Utc::now().to_rfc3339());
    applicant_friend.insert("status".into(), "active".into());
    if let Some(reason) = approved_reason.clone() {
        applicant_friend.insert("approve-reason".into(), reason);
    }
    append_record(&mut applicant_friends, applicant_friend);

    let mut approver_friend = std::collections::HashMap::new();
    approver_friend.insert("friend-id".into(), applicant_user_id.to_string());
    approver_friend.insert("add-time".into(), chrono::Utc::now().to_rfc3339());
    approver_friend.insert("status".into(), "active".into());
    if let Some(reason) = approved_reason {
        approver_friend.insert("approve-reason".into(), reason);
    }
    append_record(&mut approver_friends, approver_friend);

    applicant_sent = serialize_records(&sent_recs);
    approver_pending = serialize_records(&pending_recs);
    applicant_owned = serialize_records(&applicant_friends);
    approver_owned = serialize_records(&approver_friends);

    sqlx::query(r#"UPDATE "users" SET "user-sent-friend-requests"=$1, "user-owned-friends"=$2 WHERE "user-id"=$3"#)
        .bind(&applicant_sent)
        .bind(&applicant_owned)
        .bind(&applicant_user_id)
        .execute(&state.db)
        .await
        .map_err(|_| AuthError::InternalServerError)?;

    sqlx::query(r#"UPDATE "users" SET "user-pending-friend-requests"=$1, "user-owned-friends"=$2 WHERE "user-id"=$3"#)
        .bind(&approver_pending)
        .bind(&approver_owned)
        .bind(&approver_user_id)
        .execute(&state.db)
        .await
        .map_err(|_| AuthError::InternalServerError)?;

    Ok(())
}

async fn auto_approve(
    state: &FriendsState,
    applicant_user_id: &str,
    target_user_id: &str,
    reason: Option<String>,
) -> Result<(), AuthError> {
    manual_approve(state, target_user_id, applicant_user_id, reason).await
}