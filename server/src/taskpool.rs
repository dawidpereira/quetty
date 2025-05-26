pub(crate) use std::sync::Arc;

use futures_util::Future;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct TaskPool {
    semaphore: Arc<Semaphore>,
    cancel_token: CancellationToken,
}

impl TaskPool {
    pub fn new(n_tasks: usize) -> TaskPool {
        let semaphore = Arc::new(Semaphore::new(n_tasks));
        let cancel_token = CancellationToken::new();

        TaskPool {
            semaphore,
            cancel_token,
        }
    }

    pub fn execute<F, T>(&self, func: F)
    where
        F: Future<Output = T> + Send + 'static,
        T: Send,
    {
        let semaphore = self.semaphore.clone();
        let token = self.cancel_token.clone();

        log::debug!(
            "TaskPool: Spawning new task, available permits: {}",
            semaphore.available_permits()
        );

        tokio::spawn(async move {
            log::debug!("TaskPool: Task spawned, attempting to acquire semaphore permit");

            let main = async {
                match semaphore.acquire().await {
                    Ok(_permit) => {
                        log::debug!("TaskPool: Semaphore permit acquired, executing task");
                        func.await;
                        log::debug!("TaskPool: Task execution completed");
                    }
                    Err(e) => {
                        log::error!("TaskPool: Failed to acquire semaphore: {}", e);
                        eprintln!("Failed to acquire semaphore: {}", e);
                    }
                }
            };

            tokio::select! {
                () = main => {
                    log::debug!("TaskPool: Task finished normally");
                },
                () = token.cancelled() => {
                    log::debug!("TaskPool: Task cancelled");
                }
            }
        });

        log::debug!("TaskPool: Task submitted to tokio spawn");
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        self.semaphore.close();
        self.cancel_token.cancel();
    }
}
