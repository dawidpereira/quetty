pub(crate) use std::sync::Arc;

use futures_util::Future;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

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
        tokio::spawn(async move {
            let main = async {
                let Ok(_permit) = semaphore.acquire().await else {
                    return;
                };
                func.await;
            };

            tokio::select! {
                () = main => {},
                () = token.cancelled() => {}
            }
        });
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        self.semaphore.close();
        self.cancel_token.cancel();
    }
}
