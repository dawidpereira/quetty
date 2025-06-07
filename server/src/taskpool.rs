use futures_util::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct TaskPool {
    semaphore: Arc<Semaphore>,
    cancel_token: Arc<CancellationToken>,
}

impl TaskPool {
    pub fn new(n_tasks: usize) -> TaskPool {
        TaskPool {
            semaphore: Arc::new(Semaphore::new(n_tasks)),
            cancel_token: Arc::new(CancellationToken::new()),
        }
    }

    pub fn execute<F, T>(&self, func: F)
    where
        F: Future<Output = T> + Send + 'static,
        T: Send,
    {
        let semaphore = self.semaphore.clone();
        let token = self.cancel_token.clone();

        tokio::spawn(async move {
            let main = async {
                if let Ok(_permit) = semaphore.acquire().await {
                    func.await;
                } else {
                    log::error!("TaskPool: Failed to acquire semaphore permit");
                }
            };

            tokio::select! {
                () = main => {},
                () = token.cancelled() => {
                    log::debug!("TaskPool: Task cancelled");
                }
            }
        });
    }

    /// Cancel all running tasks
    pub fn cancel_all(&self) {
        self.cancel_token.cancel();
    }

    /// Close the semaphore to prevent new tasks from acquiring permits
    pub fn close(&self) {
        self.semaphore.close();
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        // Only close the semaphore when the last reference is dropped
        if Arc::strong_count(&self.semaphore) == 1 {
            self.semaphore.close();
        }
    }
}
