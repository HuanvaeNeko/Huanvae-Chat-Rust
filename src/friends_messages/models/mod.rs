// 数据模型模块

pub mod message;
pub mod request;
pub mod response;

pub use message::{Message, MessageResponse, MessageType};
pub use request::{DeleteMessageRequest, GetMessagesQuery, RecallMessageRequest, SendMessageRequest};
pub use response::{MessagesListResponse, SendMessageResponse, SuccessResponse};

