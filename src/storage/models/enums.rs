use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// 文件类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    Avatar,              // 头像（< 5MB）
    UserImage,           // 用户图片（< 100MB）
    UserImageFile,       // 图片文件模式（100MB - 15GB）
    UserVideo,           // 用户视频（< 15GB）
    UserVideoFile,       // 视频文件模式（15GB - 30GB）
    UserDocument,        // 用户文档（< 15GB）
    FriendImage,         // 好友聊天图片
    FriendImageFile,     // 好友聊天图片文件模式
    FriendVideo,         // 好友聊天视频
    FriendVideoFile,     // 好友聊天视频文件模式
    FriendDocument,      // 好友聊天文档
    GroupImage,          // 群聊图片（未来）
    GroupVideo,          // 群聊视频（未来）
    GroupDocument,       // 群聊文档（未来）
}

impl FileType {
    pub fn to_string(&self) -> String {
        match self {
            FileType::Avatar => "avatar".to_string(),
            FileType::UserImage => "user_image".to_string(),
            FileType::UserImageFile => "user_image_file".to_string(),
            FileType::UserVideo => "user_video".to_string(),
            FileType::UserVideoFile => "user_video_file".to_string(),
            FileType::UserDocument => "user_document".to_string(),
            FileType::FriendImage => "friend_image".to_string(),
            FileType::FriendImageFile => "friend_image_file".to_string(),
            FileType::FriendVideo => "friend_video".to_string(),
            FileType::FriendVideoFile => "friend_video_file".to_string(),
            FileType::FriendDocument => "friend_document".to_string(),
            FileType::GroupImage => "group_image".to_string(),
            FileType::GroupVideo => "group_video".to_string(),
            FileType::GroupDocument => "group_document".to_string(),
        }
    }
}

/// 存储位置枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StorageLocation {
    Avatars,          // avatars bucket
    UserFiles,        // user-file bucket
    FriendMessages,   // friends-file bucket
    GroupFiles,       // group-file bucket
}

impl StorageLocation {
    pub fn to_bucket_name(&self) -> &'static str {
        match self {
            StorageLocation::Avatars => "avatars",
            StorageLocation::UserFiles => "user-file",
            StorageLocation::FriendMessages => "friends-file",
            StorageLocation::GroupFiles => "group-file",
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            StorageLocation::Avatars => "avatars".to_string(),
            StorageLocation::UserFiles => "user_files".to_string(),
            StorageLocation::FriendMessages => "friend_messages".to_string(),
            StorageLocation::GroupFiles => "group_files".to_string(),
        }
    }
}

/// 上传模式枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UploadMode {
    OneTimeToken,     // 一次性Token（< 15GB）
    PresignedUrl,     // 预签名URL（> 15GB）
}

/// 预览支持枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PreviewSupport {
    InlinePreview,    // 支持在线预览
    DownloadOnly,     // 仅支持下载
}

impl PreviewSupport {
    pub fn to_string(&self) -> String {
        match self {
            PreviewSupport::InlinePreview => "inline_preview".to_string(),
            PreviewSupport::DownloadOnly => "download_only".to_string(),
        }
    }
}

/// 为StorageLocation实现FromStr，用于从字符串解析
impl FromStr for StorageLocation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "avatars" => Ok(StorageLocation::Avatars),
            "user_files" => Ok(StorageLocation::UserFiles),
            "friend_messages" => Ok(StorageLocation::FriendMessages),
            "group_files" => Ok(StorageLocation::GroupFiles),
            _ => Err(format!("无效的存储位置: {}", s)),
        }
    }
}

