//! # Task Pool Module
//!
//! Provides a concurrent task execution pool with semaphore-based concurrency control
//! and cancellation support. The TaskPool allows limiting the number of concurrent
//! tasks while providing graceful shutdown capabilities.
//!
//! ## Features
//!
//! - **Concurrency Control** - Limits the number of simultaneously executing tasks
//! - **Cancellation Support** - Can cancel all running tasks gracefully
//! - **Resource Management** - Automatic cleanup and resource disposal
//! - **Clone Support** - Multiple references to the same task pool
//!
//! ## Usage
//!
//! ```no_run
//! use quetty_server::taskpool::TaskPool;
//!
//! async fn example() {
//!     let pool = TaskPool::new(10); // Allow up to 10 concurrent tasks
//!
//!     // Execute multiple tasks
//!     for i in 0..20 {
//!         pool.execute(async move {
//!             println!("Task {} executing", i);
//!             // Simulate work
//!             tokio::time::sleep(std::time::Duration::from_millis(100)).await;
//!         });
//!     }
//!
//!     // Later, cancel all tasks if needed
//!     pool.cancel_all();
//! }
//! ```

use futures_util::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

/// A concurrent task execution pool with semaphore-based concurrency control.
///
/// TaskPool manages the execution of asynchronous tasks with a configurable limit
/// on the number of concurrent tasks. It provides cancellation support and automatic
/// resource cleanup.
///
/// # Thread Safety
///
/// TaskPool is thread-safe and can be cloned to share across multiple contexts.
/// All clones share the same underlying semaphore and cancellation token.
#[derive(Clone)]
pub struct TaskPool {
    semaphore: Arc<Semaphore>,
    cancel_token: Arc<CancellationToken>,
}

impl TaskPool {
    /// Creates a new TaskPool with the specified concurrency limit.
    ///
    /// # Arguments
    ///
    /// * `n_tasks` - Maximum number of tasks that can execute concurrently
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::taskpool::TaskPool;
    ///
    /// let pool = TaskPool::new(5); // Allow up to 5 concurrent tasks
    /// ```
    pub fn new(n_tasks: usize) -> TaskPool {
        TaskPool {
            semaphore: Arc::new(Semaphore::new(n_tasks)),
            cancel_token: Arc::new(CancellationToken::new()),
        }
    }

    /// Executes a future in the task pool with concurrency control.
    ///
    /// The task will wait for a semaphore permit before executing. If the pool
    /// is cancelled while the task is running, it will be interrupted gracefully.
    ///
    /// # Type Parameters
    ///
    /// * `F` - Future type that implements Send and has a static lifetime
    /// * `T` - Output type of the future that implements Send
    ///
    /// # Arguments
    ///
    /// * `func` - The async function/future to execute
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::taskpool::TaskPool;
    ///
    /// async fn example() {
    ///     let pool = TaskPool::new(3);
    ///
    ///     pool.execute(async {
    ///         println!("Task is running");
    ///         // Do some work
    ///     });
    /// }
    /// ```
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

    /// Cancels all currently running and queued tasks.
    ///
    /// This sends a cancellation signal to all tasks. Tasks that are currently
    /// executing will be interrupted at their next cancellation check point.
    /// Tasks waiting for permits will be cancelled before they start.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::taskpool::TaskPool;
    ///
    /// async fn example() {
    ///     let pool = TaskPool::new(3);
    ///
    ///     // Start some tasks
    ///     for i in 0..10 {
    ///         pool.execute(async move {
    ///             println!("Task {}", i);
    ///         });
    ///     }
    ///
    ///     // Cancel all tasks
    ///     pool.cancel_all();
    /// }
    /// ```
    pub fn cancel_all(&self) {
        self.cancel_token.cancel();
    }

    /// Closes the task pool to prevent new tasks from starting.
    ///
    /// This closes the underlying semaphore, which prevents new tasks from
    /// acquiring permits. Tasks that are already running will continue to completion.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::taskpool::TaskPool;
    ///
    /// async fn example() {
    ///     let pool = TaskPool::new(3);
    ///
    ///     // Use the pool...
    ///
    ///     // Close it to prevent new tasks
    ///     pool.close();
    /// }
    /// ```
    pub fn close(&self) {
        self.semaphore.close();
    }
}

impl Drop for TaskPool {
    /// Automatically closes the semaphore when the last TaskPool reference is dropped.
    ///
    /// This ensures that resources are properly cleaned up when the task pool
    /// is no longer needed. The semaphore is only closed when this is the last
    /// remaining reference to prevent premature shutdown.
    fn drop(&mut self) {
        // Only close the semaphore when the last reference is dropped
        if Arc::strong_count(&self.semaphore) == 1 {
            self.semaphore.close();
        }
    }
}
