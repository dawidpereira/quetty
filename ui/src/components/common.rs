use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::model::MessageModel;
use server::service_bus_manager::QueueInfo;
use std::fmt;

// Re-export QueueType from service bus instead of defining locally
pub use server::service_bus_manager::QueueType;

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
    PageSizePopup,
    HelpScreen,
    ThemePicker,
    TextLabel,
}

impl fmt::Display for ComponentId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ComponentId::TextLabel => write!(f, "TextLabel"),
            ComponentId::NamespacePicker => write!(f, "NamespacePicker"),
            ComponentId::QueuePicker => write!(f, "QueuePicker"),
            ComponentId::Messages => write!(f, "Messages"),
            ComponentId::MessageDetails => write!(f, "MessageDetails"),
            ComponentId::GlobalKeyWatcher => write!(f, "GlobalKeyWatcher"),
            ComponentId::LoadingIndicator => write!(f, "LoadingIndicator"),
            ComponentId::ConfirmationPopup => write!(f, "ConfirmationPopup"),
            ComponentId::ErrorPopup => write!(f, "ErrorPopup"),
            ComponentId::SuccessPopup => write!(f, "SuccessPopup"),
            ComponentId::HelpScreen => write!(f, "HelpScreen"),
            ComponentId::NumberInputPopup => write!(f, "NumberInputPopup"),
            ComponentId::PageSizePopup => write!(f, "PageSizePopup"),
            ComponentId::ThemePicker => write!(f, "ThemePicker"),
        }
    }
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
    ShowError(String),
    ClipboardError(String),
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
    QueueSwitchCancelled,
}

#[derive(Debug, PartialEq)]
pub enum MessageActivityMsg {
    EditMessage(usize),
    PreviewMessageDetails(usize),
    CancelEditMessage,
    MessagesLoaded(Vec<MessageModel>),
    QueueSwitched(QueueInfo),
    QueueNameUpdated(String),
    NextPage,
    PreviousPage,

    NewMessagesLoaded(Vec<MessageModel>),
    QueueStatsUpdated(crate::app::updates::messages::pagination::QueueStatsCache),
    ToggleMessageSelectionByIndex(usize),
    SelectAllCurrentPage,
    SelectAllLoadedMessages,
    ClearAllSelections,
    BulkDeleteSelected,
    BulkSendSelectedToDLQWithDelete,
    BulkResendSelectedFromDLQ(bool),
    BulkDeleteMessages(Vec<MessageIdentifier>),
    BulkSendToDLQWithDelete(Vec<MessageIdentifier>),
    BulkResendFromDLQ(Vec<MessageIdentifier>, bool),
    BulkRemoveMessagesFromState(Vec<String>),
    SendEditedMessage(String),
    ReplaceEditedMessage(String, MessageIdentifier),
    ReplaceEditedMessageConfirmed(String, MessageIdentifier, usize),
    ComposeNewMessage,
    SetMessageRepeatCount,
    UpdateRepeatCount(usize),
    MessagesSentSuccessfully,
    EditingModeStarted,
    EditingModeStopped,

    BulkDeleteCompleted {
        successful_count: usize,
        failed_count: usize,
        total_count: usize,
    },
    ForceReloadMessages,
    RefreshQueueStatistics,
}

#[derive(Debug, PartialEq)]
pub enum LoadingActivityMsg {
    Start(String),
    Stop,
    Update(String),
    Cancel,
    ShowCancelButton(String), // Show cancel button with operation ID
    HideCancelButton,
}

#[derive(Debug)]
pub enum PopupActivityMsg {
    ShowError(AppError),
    CloseError,
    ShowWarning(String),
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
    ShowPageSizePopup,
    NumberInputResult(usize),
    PageSizeResult(usize),
    ConfirmationResult(bool),
    ClosePageSize,
}

impl PartialEq for PopupActivityMsg {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PopupActivityMsg::ShowError(e1), PopupActivityMsg::ShowError(e2)) => e1 == e2,
            (PopupActivityMsg::CloseError, PopupActivityMsg::CloseError) => true,
            (PopupActivityMsg::ShowWarning(w1), PopupActivityMsg::ShowWarning(w2)) => w1 == w2,
            (PopupActivityMsg::ShowSuccess(s1), PopupActivityMsg::ShowSuccess(s2)) => s1 == s2,
            (PopupActivityMsg::CloseSuccess, PopupActivityMsg::CloseSuccess) => true,
            (PopupActivityMsg::ClosePageSize, PopupActivityMsg::ClosePageSize) => true,
            (
                PopupActivityMsg::ConfirmationResult(b1),
                PopupActivityMsg::ConfirmationResult(b2),
            ) => b1 == b2,
            (PopupActivityMsg::NumberInputResult(n1), PopupActivityMsg::NumberInputResult(n2)) => {
                n1 == n2
            }
            (PopupActivityMsg::PageSizeResult(p1), PopupActivityMsg::PageSizeResult(p2)) => {
                p1 == p2
            }
            // ShowConfirmation, ShowNumberInput, and ShowPageSizePopup are not compared due to Box types
            _ => false,
        }
    }
}

impl Default for Msg {
    fn default() -> Self {
        Self::AppClose
    }
}
