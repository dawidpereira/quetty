use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::consumer::Consumer;
use server::model::MessageModel;

#[derive(Debug, Clone, PartialEq)]
pub enum QueueType {
    Main,
    DeadLetter,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum ComponentId {
    GlobalKeyWatcher,
    NamespacePicker,
    QueuePicker,
    Messages,
    MessageDetails,
    LoadingIndicator,
    ErrorPopup,
    SuccessPopup,
    ConfirmationPopup,
    NumberInputPopup,
    HelpScreen,
    ThemePicker,
    TextLabel,
}

#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    ForceRedraw,

    MessageActivity(MessageActivityMsg),
    QueueActivity(QueueActivityMsg),
    NamespaceActivity(NamespaceActivityMsg),
    ThemeActivity(ThemeActivityMsg),
    LoadingActivity(LoadingActivityMsg),
    PopupActivity(PopupActivityMsg),
    Error(AppError),
    ToggleHelpScreen,
    ToggleThemePicker,
}

#[derive(Debug, PartialEq)]
pub enum NamespaceActivityMsg {
    NamespaceSelected,
    NamespaceUnselected,
    NamespacesLoaded(Vec<String>),
}

#[derive(Debug, PartialEq)]
pub enum ThemeActivityMsg {
    ThemeSelected(String, String), // theme_name, flavor_name
    ThemePickerClosed,
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
    QueueNameUpdated(String), // Update current queue name after consumer creation
    NextPage,
    PreviousPage,
    PaginationStateUpdated {
        has_next: bool,
        has_previous: bool,
        current_page: usize,
        total_pages_loaded: usize,
    },
    NewMessagesLoaded(Vec<MessageModel>), // New messages loaded from API
    BackfillMessagesLoaded(Vec<MessageModel>), // Messages loaded for backfilling current page
    PageChanged,                          // Just changed page within already loaded messages
    // Bulk selection messages
    ToggleMessageSelectionByIndex(usize), // Helper for UI components
    SelectAllCurrentPage,
    SelectAllLoadedMessages,
    ClearAllSelections,

    // Bulk operations - use currently selected messages
    BulkDeleteSelected,
    BulkSendSelectedToDLQ,
    BulkResendSelectedFromDLQ(bool), // bool: true = delete from DLQ, false = keep in DLQ

    // Bulk operations - with specific message lists
    BulkDeleteMessages(Vec<MessageIdentifier>),
    BulkSendToDLQ(Vec<MessageIdentifier>),
    BulkResendFromDLQ(Vec<MessageIdentifier>, bool), // bool: true = delete from DLQ, false = keep in DLQ

    // Bulk state management - remove multiple messages from local state
    BulkRemoveMessagesFromState(Vec<MessageIdentifier>),

    // Message editing operations
    SendEditedMessage(String), // Send edited content as new message
    ReplaceEditedMessage(String, MessageIdentifier), // Replace original message with edited content

    // Message composition operations
    ComposeNewMessage,        // Open empty message details in edit mode
    SetMessageRepeatCount,    // Open popup to set how many times to send message
    UpdateRepeatCount(usize), // Internal: Update the repeat count value
    MessagesSentSuccessfully, // Trigger auto-reload after successful message sending

    // Message editing mode state tracking
    EditingModeStarted, // Notify that message details entered edit mode
    EditingModeStopped, // Notify that message details exited edit mode
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
    ShowWarning(String),
    #[allow(dead_code)]
    CloseWarning,
    ShowSuccess(String),
    CloseSuccess,
    ShowConfirmation {
        title: String,
        message: String,
        on_confirm: Box<Msg>,
    },
    ShowNumberInput {
        title: String,
        message: String,
        min_value: usize,
        max_value: usize,
    },
    NumberInputResult(usize),
    ConfirmationResult(bool),
}

impl PartialEq for PopupActivityMsg {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PopupActivityMsg::ShowError(e1), PopupActivityMsg::ShowError(e2)) => e1 == e2,
            (PopupActivityMsg::CloseError, PopupActivityMsg::CloseError) => true,
            (PopupActivityMsg::ShowWarning(w1), PopupActivityMsg::ShowWarning(w2)) => w1 == w2,
            (PopupActivityMsg::CloseWarning, PopupActivityMsg::CloseWarning) => true,
            (PopupActivityMsg::ShowSuccess(s1), PopupActivityMsg::ShowSuccess(s2)) => s1 == s2,
            (PopupActivityMsg::CloseSuccess, PopupActivityMsg::CloseSuccess) => true,
            (
                PopupActivityMsg::ConfirmationResult(b1),
                PopupActivityMsg::ConfirmationResult(b2),
            ) => b1 == b2,
            (PopupActivityMsg::NumberInputResult(n1), PopupActivityMsg::NumberInputResult(n2)) => {
                n1 == n2
            }
            // ShowConfirmation and ShowNumberInput are not compared due to Box types
            _ => false,
        }
    }
}

impl Default for Msg {
    fn default() -> Self {
        Self::AppClose
    }
}
