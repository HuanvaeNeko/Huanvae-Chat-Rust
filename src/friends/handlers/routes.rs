use axum::{middleware, routing::{get, post}, Router};
use crate::auth::middleware::{auth_guard, AuthState};
use crate::friends::handlers::create_request::create_friend_request_handler;
use crate::friends::handlers::approve_request::approve_friend_request_handler;
use crate::friends::handlers::reject_request::reject_friend_request_handler;
use crate::friends::handlers::list_pending::ListState as PendingListState;
use crate::friends::handlers::list_sent::ListState as SentListState;
use crate::friends::handlers::list_owned::ListState as OwnedListState;
use crate::friends::handlers::remove_friend::remove_friend_handler;
use crate::friends::services::FriendsState;
use crate::friends::handlers::list_pending::list_pending_requests_handler;
use crate::friends::handlers::list_sent::list_sent_requests_handler;
use crate::friends::handlers::list_owned::list_owned_friends_handler;

pub fn create_friend_routes(
    friends_state: FriendsState,
    auth_state: AuthState,
    db: sqlx::PgPool,
) -> Router {
    let write = Router::new()
        .route("/requests", post(create_friend_request_handler))
        .route("/requests/approve", post(approve_friend_request_handler))
        .route("/requests/reject", post(reject_friend_request_handler))
        .route("/remove", post(remove_friend_handler))
        .with_state(friends_state.clone())
        .layer(middleware::from_fn_with_state(auth_state.clone(), auth_guard));

    let read = Router::new()
        .route("/requests/sent", get(list_sent_requests_handler))
        .with_state(SentListState { db: db.clone() })
        .route("/requests/pending", get(list_pending_requests_handler))
        .with_state(PendingListState { db: db.clone() })
        .route("/", get(list_owned_friends_handler))
        .with_state(OwnedListState { db: db.clone() })
        .layer(middleware::from_fn_with_state(auth_state, auth_guard));

    Router::new().merge(write).merge(read)
}