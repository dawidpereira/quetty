use crate::app::managers::{QueueManager, StateManager};
// Re-export AppState for other modules
pub use crate::app::managers::state_manager::AppState;
use crate::app::task_manager::TaskManager;
use crate::components::common::{ComponentId, Msg};
use crate::error::AppError;
use crate::error::ErrorReporter;
use crate::services::AuthService;
use server::service_bus_manager::ServiceBusManager;
use server::service_bus_manager::{ServiceBusCommand, ServiceBusResponse};
use server::taskpool::TaskPool;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use tokio::sync::Mutex;
use tuirealm::event::NoUserEvent;
use tuirealm::terminal::{TerminalAdapter, TerminalBridge};
use tuirealm::{Application, Update};

// Submodules
mod async_operations;
mod initialization;
mod popup_management;
mod state_management;
mod update_handler;

/// Application model using composition with managers
pub struct Model<T>
where
    T: TerminalAdapter,
{
    /// Application
    pub app: Application<ComponentId, Msg, NoUserEvent>,
    /// Used to draw to terminal
    pub terminal: TerminalBridge<T>,

    pub taskpool: TaskPool,
    pub rx_to_main: Receiver<Msg>,

    /// Service bus manager - direct access to server-side manager
    pub service_bus_manager: Arc<Mutex<ServiceBusManager>>,

    // Enhanced error reporting system
    pub error_reporter: ErrorReporter,

    // Task manager for consistent async operations
    pub task_manager: TaskManager,

    // Managers for different concerns
    pub state_manager: StateManager,
    pub queue_manager: QueueManager,

    // Authentication service
    pub auth_service: Option<Arc<AuthService>>,
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

        // Set quit flag through state manager
        self.state_manager.shutdown();

        // Dispose service bus resources
        let service_bus_manager = self.service_bus_manager.clone();
        self.task_manager
            .execute("Disposing service bus resources...", async move {
                let command = ServiceBusCommand::DisposeAllResources;
                let response = service_bus_manager
                    .lock()
                    .await
                    .execute_command(command)
                    .await;
                match response {
                    ServiceBusResponse::AllResourcesDisposed => Ok(()),
                    ServiceBusResponse::Error { error } => {
                        Err(AppError::ServiceBus(error.to_string()))
                    }
                    _ => Err(AppError::ServiceBus("Unexpected response".to_string())),
                }
            });
    }

    // Essential accessor methods

    /// Get immutable reference to queue state
    pub fn queue_state(&self) -> &crate::app::queue_state::QueueState {
        &self.queue_manager.queue_state
    }

    /// Get mutable reference to queue state
    pub fn queue_state_mut(&mut self) -> &mut crate::app::queue_state::QueueState {
        &mut self.queue_manager.queue_state
    }

    /// Set app state
    pub fn set_app_state(&mut self, state: AppState) {
        self.state_manager.set_app_state(state);
    }

    /// Set editing message mode
    pub fn set_editing_message(&mut self, editing: bool) {
        self.state_manager.set_editing_message(editing);
    }

    /// Set redraw flag
    pub fn set_redraw(&mut self, redraw: bool) {
        self.state_manager.redraw = redraw;
    }

    /// Set quit flag
    pub fn set_quit(&mut self, quit: bool) {
        if quit {
            self.state_manager.quit = true;
        }
    }

    /// Get tx_to_main sender
    pub fn tx_to_main(&self) -> &std::sync::mpsc::Sender<Msg> {
        &self.state_manager.tx_to_main
    }

    /// Set selected namespace
    pub fn set_selected_namespace(&mut self, namespace: Option<String>) {
        self.state_manager.selected_namespace = namespace;
    }

    /// Set pending confirmation action
    pub fn set_pending_confirmation_action(&mut self, action: Option<Box<Msg>>) {
        self.state_manager.pending_confirmation_action = action;
    }

    /// Take pending confirmation action
    pub fn take_pending_confirmation_action(&mut self) -> Option<Box<Msg>> {
        self.state_manager.take_pending_confirmation()
    }

    /// Set active component
    pub fn set_active_component(&mut self, component: ComponentId) {
        self.state_manager.set_active_component(component);
    }

    /// Get current page size (dynamic or from config)
    pub fn get_current_page_size(&self) -> u32 {
        self.state_manager.get_current_page_size()
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
