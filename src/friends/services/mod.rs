pub mod friends_service;

pub use friends_service::FriendsService;
pub use friends_service::{
    approve_request, reject_request, remove_friend, submit_request, verify_friendship,
};
