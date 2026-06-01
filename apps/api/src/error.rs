use axum::{
    Json,
    extract::multipart::MultipartError,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::error::Error as StdError;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("认证已失效，请重新登录")]
    Unauthorized,
    #[error("请求来源不可信")]
    Forbidden,
    #[error("重复提交，请稍后再试")]
    DuplicateSubmit,
    #[error("没有找到资源")]
    NotFound,
    #[error("资源已存在：{0}")]
    Conflict(String),
    #[error("请求参数无效：{0}")]
    BadRequest(String),
    #[error("GitHub 请求失败：{0}")]
    GitHub(String),
    #[error("服务内部错误：{0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AppError {
    /// 将内部错误转换成统一的 JSON 响应。
    fn into_response(self) -> Response {
        let status = match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::DuplicateSubmit => StatusCode::CONFLICT,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::GitHub(_) => StatusCode::BAD_GATEWAY,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message = self.to_string();

        (status, Json(ErrorBody { error: message })).into_response()
    }
}

impl From<reqwest::Error> for AppError {
    /// 将 reqwest 错误归类为 GitHub 上游错误。
    fn from(err: reqwest::Error) -> Self {
        let mut message = err.to_string();
        let mut source = err.source();

        while let Some(err) = source {
            message.push_str(": ");
            message.push_str(&err.to_string());
            source = err.source();
        }

        Self::GitHub(message)
    }
}

impl From<serde_json::Error> for AppError {
    /// 将 JSON 解析错误归类为上游数据格式错误。
    fn from(err: serde_json::Error) -> Self {
        Self::GitHub(format!("JSON 格式无效：{err}"))
    }
}

impl From<base64::DecodeError> for AppError {
    /// 将 Base64 解码错误归类为上游数据格式错误。
    fn from(err: base64::DecodeError) -> Self {
        Self::GitHub(format!("Base64 内容无效：{err}"))
    }
}

impl From<MultipartError> for AppError {
    /// 将 multipart 解析错误归类为上传参数错误。
    fn from(err: MultipartError) -> Self {
        Self::BadRequest(format!("上传数据无效：{err}"))
    }
}
