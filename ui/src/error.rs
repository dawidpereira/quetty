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
            _ => false,
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn handle_error(error: AppError) {
    // Log the error with appropriate level based on error type
    match &error {
        AppError::Io(msg) => log::error!("IO Error: {}", msg),
        AppError::ServiceBus(msg) => log::error!("Service Bus Error: {}", msg),
        AppError::Component(msg) => log::warn!("Component Error: {}", msg),
        AppError::State(msg) => log::warn!("State Error: {}", msg),
    }

    // Print to stderr (this will be redundant with logging, but we'll keep it for now)
    // TODO: Display error in UI
    eprintln!("Error: {:?}", error);
}
