use crate::app::queue_state::QueueState;
use crate::components::common::{ComponentId, Msg};
use azservicebus::ServiceBusClient;
use azservicebus::core::BasicRetryPolicy;
use server::taskpool::TaskPool;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use tuirealm::event::NoUserEvent;
use tuirealm::terminal::{TerminalAdapter, TerminalBridge};
use tuirealm::{Application, Update};

// Submodules
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

    pub service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    pub active_component: ComponentId,

    // All queue-related state
    pub queue_state: QueueState,

    // Pending confirmation action (message to execute on confirmation)
    pub pending_confirmation_action: Option<Box<Msg>>,

    // Track if we're currently in message editing mode
    pub is_editing_message: bool,
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_outside_msg(&mut self) {
        // Handle messages sent from background tasks
        while let Ok(msg) = self.rx_to_main.try_recv() {
            if let Some(msg) = self.update(Some(msg)) {
                let _ = self.update(Some(msg));
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
