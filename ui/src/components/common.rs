use server::consumer::Consumer;
use server::model::MessageModel;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq)]
pub enum QueueType {
    Main,
    DeadLetter,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum ComponentId {
    Label,
    Messages,
    MessageDetails,
    QueuePicker,
    NamespacePicker,
    GlobalKeyWatcher,
    ErrorPopup,
    ConfirmationPopup,
    LoadingIndicator,
    HelpScreen,
}

#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    ForceRedraw,
    Submit(Vec<String>),
    MessageActivity(MessageActivityMsg),
    QueueActivity(QueueActivityMsg),
    NamespaceActivity(NamespaceActivityMsg),
    LoadingActivity(LoadingActivityMsg),
    PopupActivity(PopupActivityMsg),
    Error(AppError),
    ToggleHelpScreen,
}

#[derive(Debug, PartialEq)]
pub enum NamespaceActivityMsg {
    NamespaceSelected,
    NamespaceUnselected,
    NamespacesLoaded(Vec<String>),
}

#[derive(Debug, PartialEq)]
pub enum QueueActivityMsg {
    QueueSelected(String),
    QueueUnselected,
    QueuesLoaded(Vec<String>),
    ToggleDeadLetterQueue,
}

#[derive(Debug, PartialEq)]
pub enum MessageActivityMsg {
    EditMessage(usize),
    PreviewMessageDetails(usize),
    CancelEditMessage,
    MessagesLoaded(Vec<MessageModel>),
    ConsumerCreated(Consumer),
    NextPage,
    PreviousPage,
    PaginationStateUpdated {
        has_next: bool,
        has_previous: bool,
        current_page: usize,
        total_pages_loaded: usize,
    },
    NewMessagesLoaded(Vec<MessageModel>), // New messages loaded from API
    PageChanged,                          // Just changed page within already loaded messages
    SendMessageToDLQ(usize),              // Send message at index to dead letter queue
    RemoveMessageFromState(String, i64), // Remove message by ID and sequence from local state (after DLQ)
}

#[derive(Debug, PartialEq)]
pub enum LoadingActivityMsg {
    Start(String),
    Update(String),
    Stop,
}

#[derive(Debug)]
pub enum PopupActivityMsg {
    ShowError(AppError),
    CloseError,
    ShowConfirmation {
        title: String,
        message: String,
        on_confirm: Box<Msg>,
    },
    ConfirmationResult(bool),
}

impl PartialEq for PopupActivityMsg {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PopupActivityMsg::ShowError(e1), PopupActivityMsg::ShowError(e2)) => e1 == e2,
            (PopupActivityMsg::CloseError, PopupActivityMsg::CloseError) => true,
            (
                PopupActivityMsg::ConfirmationResult(b1),
                PopupActivityMsg::ConfirmationResult(b2),
            ) => b1 == b2,
            // ShowConfirmation is not compared due to Box<Msg>
            _ => false,
        }
    }
}

impl Default for Msg {
    fn default() -> Self {
        Self::AppClose
    }
}
