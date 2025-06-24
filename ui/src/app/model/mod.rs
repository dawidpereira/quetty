use crate::app::queue_state::QueueState;
use crate::app::task_manager::TaskManager;
use crate::components::common::{ComponentId, Msg};
use crate::error::ErrorReporter;
use server::service_bus_manager::ServiceBusManager;
use server::taskpool::TaskPool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use tokio::sync::Mutex;
use tuirealm::event::NoUserEvent;
use tuirealm::terminal::{TerminalAdapter, TerminalBridge};
use tuirealm::{Application, Update};
use server::service_bus_manager::{ServiceBusCommand, ServiceBusResponse};
use crate::error::AppError;

// Submodules
mod async_operations;
mod initialization;
mod popup_management;
mod state_management;
mod update_handler;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    NamespacePicker,
    QueuePicker,
    MessagePicker,
    MessageDetails,
    Loading,
    HelpScreen,
    ThemePicker,
}

/// Application model
pub struct Model<T>
where
    T: TerminalAdapter,
{
    /// Application
    pub app: Application<ComponentId, Msg, NoUserEvent>,
    pub app_state: AppState,
    /// Indicates that the application must quit
    pub quit: bool,
    /// Tells whether to redraw interface
    pub redraw: bool,
    /// Used to draw to terminal
    pub terminal: TerminalBridge<T>,

    pub selected_namespace: Option<String>,
    // Store both the loading message and the previous state to return to
    pub loading_message: Option<(String, AppState)>,
    // Store the previous state when showing help screen
    pub previous_state: Option<AppState>,

    pub taskpool: TaskPool,
    pub tx_to_main: Sender<Msg>,
    pub rx_to_main: Receiver<Msg>,

    /// Service bus manager - direct access to server-side manager
    pub service_bus_manager: Arc<Mutex<ServiceBusManager>>,
    pub active_component: ComponentId,

    // All queue-related state
    pub queue_state: QueueState,

    // Pending confirmation action (message to execute on confirmation)
    pub pending_confirmation_action: Option<Box<Msg>>,

    // Track if we're currently in message editing mode
    pub is_editing_message: bool,

    // Enhanced error reporting system
    pub error_reporter: ErrorReporter,

    // Task manager for consistent async operations
    pub task_manager: TaskManager,
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_outside_msg(&mut self) {
        // Handle messages sent from background tasks
        while let Ok(msg) = self.rx_to_main.try_recv() {
            let mut msg = Some(msg);
            while msg.is_some() {
                msg = self.update(msg);
            }
        }
    }

    /// Shutdown the application and clean up resources
    pub fn shutdown(&mut self) {
        log::info!("Shutting down application");

        // Cancel all running tasks
        self.taskpool.cancel_all();

        // Close the semaphore to prevent new tasks
        self.taskpool.close();

        // Set quit flag
        self.quit = true;

        // Dispose service bus resources
        let service_bus_manager = self.service_bus_manager.clone();
        self.task_manager
            .execute("Disposing service bus resources...", async move {
                
                let command = ServiceBusCommand::DisposeAllResources;
                let response = service_bus_manager.lock().await.execute_command(command).await;
                match response {
                    AllResourcesDisposed => Ok(()),
                    ServiceBusResponse::Error { error } => {
                        Err(AppError::ServiceBus(error.to_string()))
                    }
                    _ => Err(AppError::ServiceBus("Unexpected response".to_string())),
                }
            });
    }
}

impl<T> Update<Msg> for Model<T>
where
    T: TerminalAdapter,
{
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        self.handle_update(msg)
    }
}
