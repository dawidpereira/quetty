use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

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

pub type AppResult<T> = Result<T, AppError>;

pub fn handle_error(error: AppError) {
    // TODO: Implement proper error handling UI
    eprintln!("Error: {:?}", error);
}

