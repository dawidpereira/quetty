use std::io;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("Service Bus error: {0}")]
    ServiceBus(String),

    #[error("Component error: {0}")]
    Component(String),

    #[error("State error: {0}")]
    State(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

impl PartialEq for AppError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(a), Self::Io(b)) => a == b,
            (Self::ServiceBus(a), Self::ServiceBus(b)) => a == b,
            (Self::Component(a), Self::Component(b)) => a == b,
            (Self::State(a), Self::State(b)) => a == b,
            (Self::Api(a), Self::Api(b)) => a == b,
            (Self::Config(a), Self::Config(b)) => a == b,
            (Self::Unknown(a), Self::Unknown(b)) => a == b,
            _ => false,
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn handle_error(error: AppError) {
    // TODO: Implement proper error handling UI
    eprintln!("Error: {:?}", error);
}
