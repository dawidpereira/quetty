use super::types::AuthType;
use crate::service_bus_manager::ServiceBusError;
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct AuthToken {
    pub token: String,
    pub token_type: String,
    pub expires_in_secs: Option<u64>,
}

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn authenticate(&self) -> Result<AuthToken, ServiceBusError>;

    async fn refresh(&self) -> Result<AuthToken, ServiceBusError> {
        self.authenticate().await
    }

    fn auth_type(&self) -> AuthType;

    fn requires_refresh(&self) -> bool {
        true
    }
}
