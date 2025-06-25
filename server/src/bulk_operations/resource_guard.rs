use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

/// RAII guard for managing service bus resources with automatic cleanup
pub struct ServiceBusResourceGuard<T: 'static> {
    resource: Option<tokio::sync::MutexGuard<'static, T>>,
    cleanup_fn: Option<Box<dyn FnOnce() + Send>>,
}

impl<T: 'static> ServiceBusResourceGuard<T> {
    /// Create a new resource guard with optional cleanup function
    pub fn new(
        resource: tokio::sync::MutexGuard<'static, T>,
        cleanup_fn: Option<Box<dyn FnOnce() + Send>>,
    ) -> Self {
        Self {
            resource: Some(resource),
            cleanup_fn,
        }
    }

    /// Get a reference to the guarded resource
    pub fn get(&self) -> Option<&T> {
        self.resource.as_deref()
    }

    /// Get a mutable reference to the guarded resource
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.resource.as_deref_mut()
    }
}

impl<T: 'static> Drop for ServiceBusResourceGuard<T> {
    fn drop(&mut self) {
        // Drop the resource first
        self.resource.take();

        // Then run cleanup if provided
        if let Some(cleanup) = self.cleanup_fn.take() {
            cleanup();
        }

        log::debug!("ServiceBusResourceGuard: Resource cleanup completed");
    }
}

/// Safe lock acquisition with timeout
pub async fn acquire_lock_with_timeout<'a, T>(
    mutex: &'a Arc<Mutex<T>>,
    operation_name: &str,
    timeout_duration: Duration,
    cancel_token: Option<&CancellationToken>,
) -> Result<tokio::sync::MutexGuard<'a, T>, Box<dyn Error + Send + Sync>> {
    log::debug!("Attempting to acquire lock for {}", operation_name);

    let lock_future = mutex.lock();

    // Handle cancellation if token is provided
    if let Some(token) = cancel_token {
        tokio::select! {
            guard = timeout(timeout_duration, lock_future) => {
                match guard {
                    Ok(guard) => {
                        log::debug!("Successfully acquired lock for {}", operation_name);
                        Ok(guard)
                    }
                    Err(_) => {
                        let error_msg = format!(
                            "Timeout acquiring lock for {} after {:?}",
                            operation_name,
                            timeout_duration
                        );
                        log::error!("{}", error_msg);
                        Err(Box::new(std::io::Error::new(std::io::ErrorKind::TimedOut, error_msg)))
                    }
                }
            }
            _ = token.cancelled() => {
                let error_msg = format!("Lock acquisition for {} was cancelled", operation_name);
                log::warn!("{}", error_msg);
                Err(Box::new(std::io::Error::new(std::io::ErrorKind::Interrupted, error_msg)))
            }
        }
    } else {
        // No cancellation token, just use timeout
        match timeout(timeout_duration, lock_future).await {
            Ok(guard) => {
                log::debug!("Successfully acquired lock for {}", operation_name);
                Ok(guard)
            }
            Err(_) => {
                let error_msg = format!(
                    "Timeout acquiring lock for {} after {:?}",
                    operation_name, timeout_duration
                );
                log::error!("{}", error_msg);
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    error_msg,
                )))
            }
        }
    }
} 