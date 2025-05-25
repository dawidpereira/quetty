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
    Error(AppError),
    CloseErrorPopup,
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
}

#[derive(Debug, PartialEq)]
pub enum LoadingActivityMsg {
    Start(String),
    Update(String),
    Stop,
}

impl Default for Msg {
    fn default() -> Self {
        Self::AppClose
    }
}
