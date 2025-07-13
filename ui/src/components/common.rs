use crate::app::updates::messages::pagination::QueueStatsCache;
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::model::MessageModel;
use server::service_bus_manager::QueueInfo;
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    ConfigScreen,
    PasswordPopup,
    TextLabel,
    AuthPopup,
    SubscriptionPicker,
    ResourceGroupPicker,
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
            ComponentId::ConfigScreen => write!(f, "ConfigScreen"),
            ComponentId::PasswordPopup => write!(f, "PasswordPopup"),
            ComponentId::AuthPopup => write!(f, "AuthPopup"),
            ComponentId::SubscriptionPicker => write!(f, "SubscriptionPicker"),
            ComponentId::ResourceGroupPicker => write!(f, "ResourceGroupPicker"),
        }
    }
}

pub enum Msg {
    AppClose,
    ForceRedraw,
    Tick,

    MessageActivity(MessageActivityMsg),
    QueueActivity(QueueActivityMsg),
    NamespaceActivity(NamespaceActivityMsg),
    ThemeActivity(ThemeActivityMsg),
    LoadingActivity(LoadingActivityMsg),
    PopupActivity(PopupActivityMsg),
    Error(AppError),
    ShowError(String),
    ShowSuccess(String),
    ClipboardError(String),
    ToggleHelpScreen,
    ToggleThemePicker,
    ToggleConfigScreen,
    TogglePasswordPopup,
    AuthActivity(AuthActivityMsg),
    ConfigActivity(ConfigActivityMsg),
    SetEditingMode(bool),
    SubscriptionSelection(SubscriptionSelectionMsg),
    ResourceGroupSelection(ResourceGroupSelectionMsg),
    AzureDiscovery(AzureDiscoveryMsg),
    SetServiceBusManager(Arc<Mutex<server::service_bus_manager::ServiceBusManager>>),
}

impl std::fmt::Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Msg::AppClose => write!(f, "AppClose"),
            Msg::ForceRedraw => write!(f, "ForceRedraw"),
            Msg::Tick => write!(f, "Tick"),
            Msg::MessageActivity(msg) => write!(f, "MessageActivity({msg:?})"),
            Msg::QueueActivity(msg) => write!(f, "QueueActivity({msg:?})"),
            Msg::NamespaceActivity(msg) => write!(f, "NamespaceActivity({msg:?})"),
            Msg::ThemeActivity(msg) => write!(f, "ThemeActivity({msg:?})"),
            Msg::LoadingActivity(msg) => write!(f, "LoadingActivity({msg:?})"),
            Msg::PopupActivity(msg) => write!(f, "PopupActivity({msg:?})"),
            Msg::Error(err) => write!(f, "Error({err:?})"),
            Msg::ShowError(msg) => write!(f, "ShowError({msg:?})"),
            Msg::ShowSuccess(msg) => write!(f, "ShowSuccess({msg:?})"),
            Msg::ClipboardError(msg) => write!(f, "ClipboardError({msg:?})"),
            Msg::ToggleHelpScreen => write!(f, "ToggleHelpScreen"),
            Msg::ToggleThemePicker => write!(f, "ToggleThemePicker"),
            Msg::ToggleConfigScreen => write!(f, "ToggleConfigScreen"),
            Msg::TogglePasswordPopup => write!(f, "TogglePasswordPopup"),
            Msg::AuthActivity(msg) => write!(f, "AuthActivity({msg:?})"),
            Msg::ConfigActivity(msg) => write!(f, "ConfigActivity({msg:?})"),
            Msg::SetEditingMode(editing) => write!(f, "SetEditingMode({editing})"),
            Msg::SubscriptionSelection(msg) => write!(f, "SubscriptionSelection({msg:?})"),
            Msg::ResourceGroupSelection(msg) => write!(f, "ResourceGroupSelection({msg:?})"),
            Msg::AzureDiscovery(msg) => write!(f, "AzureDiscovery({msg:?})"),
            Msg::SetServiceBusManager(_) => write!(f, "SetServiceBusManager(<ServiceBusManager>)"),
        }
    }
}

impl PartialEq for Msg {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Msg::AppClose, Msg::AppClose) => true,
            (Msg::ForceRedraw, Msg::ForceRedraw) => true,
            (Msg::Tick, Msg::Tick) => true,
            (Msg::MessageActivity(a), Msg::MessageActivity(b)) => a == b,
            (Msg::QueueActivity(a), Msg::QueueActivity(b)) => a == b,
            (Msg::NamespaceActivity(a), Msg::NamespaceActivity(b)) => a == b,
            (Msg::ThemeActivity(a), Msg::ThemeActivity(b)) => a == b,
            (Msg::LoadingActivity(a), Msg::LoadingActivity(b)) => a == b,
            (Msg::PopupActivity(a), Msg::PopupActivity(b)) => a == b,
            (Msg::Error(a), Msg::Error(b)) => a == b,
            (Msg::ShowError(a), Msg::ShowError(b)) => a == b,
            (Msg::ShowSuccess(a), Msg::ShowSuccess(b)) => a == b,
            (Msg::ClipboardError(a), Msg::ClipboardError(b)) => a == b,
            (Msg::ToggleHelpScreen, Msg::ToggleHelpScreen) => true,
            (Msg::ToggleThemePicker, Msg::ToggleThemePicker) => true,
            (Msg::ToggleConfigScreen, Msg::ToggleConfigScreen) => true,
            (Msg::AuthActivity(a), Msg::AuthActivity(b)) => a == b,
            (Msg::ConfigActivity(a), Msg::ConfigActivity(b)) => a == b,
            (Msg::SetEditingMode(a), Msg::SetEditingMode(b)) => a == b,
            (Msg::SubscriptionSelection(a), Msg::SubscriptionSelection(b)) => a == b,
            (Msg::ResourceGroupSelection(a), Msg::ResourceGroupSelection(b)) => a == b,
            (Msg::AzureDiscovery(a), Msg::AzureDiscovery(b)) => a == b,
            (Msg::SetServiceBusManager(_), Msg::SetServiceBusManager(_)) => false, // Never equal since we can't compare ServiceBusManager
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum AuthActivityMsg {
    Login,
    ShowDeviceCode {
        user_code: String,
        verification_url: String,
        message: String,
        expires_in: u64, // Seconds until expiry
    },
    AuthenticationSuccess,
    AuthenticationFailed(String),
    CancelAuthentication,
    CopyDeviceCode,
    OpenVerificationUrl,
    TokenRefreshFailed(String),
    CreateServiceBusManager,
}

#[derive(Debug, PartialEq)]
pub enum NamespaceActivityMsg {
    NamespaceSelected,
    NamespaceUnselected,
    NamespacesLoaded(Vec<String>),
    NamespaceCancelled,
}

#[derive(Debug, PartialEq)]
pub enum ThemeActivityMsg {
    ThemeSelected(String, String), // theme_name, flavor_name
    ThemePickerClosed,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ConfigUpdateData {
    pub auth_method: String,
    pub tenant_id: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub subscription_id: Option<String>,
    pub resource_group: Option<String>,
    pub namespace: Option<String>,
    pub connection_string: Option<String>,
    pub master_password: Option<String>,
    pub queue_name: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum ConfigActivityMsg {
    Save(ConfigUpdateData),
    ConfirmAndProceed(ConfigUpdateData),
    Cancel,
}

#[derive(Debug, PartialEq)]
pub enum QueueActivityMsg {
    QueueSelected(String),
    QueueUnselected,
    QueuesLoaded(Vec<String>),
    ToggleDeadLetterQueue,
    QueueSwitchCancelled,
    /// User requested to exit the current queue (shows confirmation dialog)
    ExitQueueConfirmation,
    /// User confirmed queue exit - triggers resource cleanup and returns to queue selection
    ExitQueueConfirmed,
    /// Resource disposal completed - finalize the exit process
    ExitQueueFinalized,
    /// Queue selected from manual entry mode - needs to exit editing mode first
    QueueSelectedFromManualEntry(String),
}

#[derive(Debug, PartialEq)]
pub enum SubscriptionSelectionMsg {
    SubscriptionSelected(String),
    SelectionChanged,
    CancelSelection,
}

#[derive(Debug, PartialEq)]
pub enum ResourceGroupSelectionMsg {
    ResourceGroupSelected(String),
    SelectionChanged,
    CancelSelection,
}

pub enum AzureDiscoveryMsg {
    StartDiscovery,
    StartInteractiveDiscovery,
    DiscoveringSubscriptions,
    SubscriptionsDiscovered(
        Vec<server::service_bus_manager::azure_management_client::Subscription>,
    ),
    DiscoveringResourceGroups(String), // subscription_id
    ResourceGroupsDiscovered(
        Vec<server::service_bus_manager::azure_management_client::ResourceGroup>,
    ),
    DiscoveringNamespaces(String), // subscription_id
    NamespacesDiscovered(
        Vec<server::service_bus_manager::azure_management_client::ServiceBusNamespace>,
    ),
    FetchingConnectionString {
        subscription_id: String,
        resource_group: String,
        namespace: String,
    },
    ConnectionStringFetched(String),
    ServiceBusManagerCreated,
    ServiceBusManagerReady(Arc<tokio::sync::Mutex<server::service_bus_manager::ServiceBusManager>>),
    DiscoveryError(String),
    DiscoveryComplete,
}

impl fmt::Debug for AzureDiscoveryMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AzureDiscoveryMsg::StartDiscovery => write!(f, "StartDiscovery"),
            AzureDiscoveryMsg::StartInteractiveDiscovery => write!(f, "StartInteractiveDiscovery"),
            AzureDiscoveryMsg::DiscoveringSubscriptions => write!(f, "DiscoveringSubscriptions"),
            AzureDiscoveryMsg::SubscriptionsDiscovered(subs) => {
                write!(f, "SubscriptionsDiscovered({} items)", subs.len())
            }
            AzureDiscoveryMsg::DiscoveringResourceGroups(id) => {
                write!(f, "DiscoveringResourceGroups({id})")
            }
            AzureDiscoveryMsg::ResourceGroupsDiscovered(groups) => {
                write!(f, "ResourceGroupsDiscovered({} items)", groups.len())
            }
            AzureDiscoveryMsg::DiscoveringNamespaces(id) => {
                write!(f, "DiscoveringNamespaces({id})")
            }
            AzureDiscoveryMsg::NamespacesDiscovered(ns) => {
                write!(f, "NamespacesDiscovered({} items)", ns.len())
            }
            AzureDiscoveryMsg::FetchingConnectionString {
                subscription_id,
                resource_group,
                namespace,
            } => {
                write!(
                    f,
                    "FetchingConnectionString {{ subscription_id: {subscription_id}, resource_group: {resource_group}, namespace: {namespace} }}"
                )
            }
            AzureDiscoveryMsg::ConnectionStringFetched(_) => {
                write!(f, "ConnectionStringFetched(...)")
            }
            AzureDiscoveryMsg::ServiceBusManagerCreated => write!(f, "ServiceBusManagerCreated"),
            AzureDiscoveryMsg::ServiceBusManagerReady(_) => {
                write!(f, "ServiceBusManagerReady(Arc<Mutex<ServiceBusManager>>)")
            }
            AzureDiscoveryMsg::DiscoveryError(e) => write!(f, "DiscoveryError({e})"),
            AzureDiscoveryMsg::DiscoveryComplete => write!(f, "DiscoveryComplete"),
        }
    }
}

impl PartialEq for AzureDiscoveryMsg {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AzureDiscoveryMsg::StartDiscovery, AzureDiscoveryMsg::StartDiscovery) => true,
            (
                AzureDiscoveryMsg::StartInteractiveDiscovery,
                AzureDiscoveryMsg::StartInteractiveDiscovery,
            ) => true,
            (
                AzureDiscoveryMsg::DiscoveringSubscriptions,
                AzureDiscoveryMsg::DiscoveringSubscriptions,
            ) => true,
            (
                AzureDiscoveryMsg::SubscriptionsDiscovered(a),
                AzureDiscoveryMsg::SubscriptionsDiscovered(b),
            ) => a == b,
            (
                AzureDiscoveryMsg::DiscoveringResourceGroups(a),
                AzureDiscoveryMsg::DiscoveringResourceGroups(b),
            ) => a == b,
            (
                AzureDiscoveryMsg::ResourceGroupsDiscovered(a),
                AzureDiscoveryMsg::ResourceGroupsDiscovered(b),
            ) => a == b,
            (
                AzureDiscoveryMsg::DiscoveringNamespaces(a),
                AzureDiscoveryMsg::DiscoveringNamespaces(b),
            ) => a == b,
            (
                AzureDiscoveryMsg::NamespacesDiscovered(a),
                AzureDiscoveryMsg::NamespacesDiscovered(b),
            ) => a == b,
            (
                AzureDiscoveryMsg::FetchingConnectionString {
                    subscription_id: a1,
                    resource_group: a2,
                    namespace: a3,
                },
                AzureDiscoveryMsg::FetchingConnectionString {
                    subscription_id: b1,
                    resource_group: b2,
                    namespace: b3,
                },
            ) => a1 == b1 && a2 == b2 && a3 == b3,
            (
                AzureDiscoveryMsg::ConnectionStringFetched(a),
                AzureDiscoveryMsg::ConnectionStringFetched(b),
            ) => a == b,
            (
                AzureDiscoveryMsg::ServiceBusManagerCreated,
                AzureDiscoveryMsg::ServiceBusManagerCreated,
            ) => true,
            (
                AzureDiscoveryMsg::ServiceBusManagerReady(_),
                AzureDiscoveryMsg::ServiceBusManagerReady(_),
            ) => false, // Can't compare managers
            (AzureDiscoveryMsg::DiscoveryError(a), AzureDiscoveryMsg::DiscoveryError(b)) => a == b,
            (AzureDiscoveryMsg::DiscoveryComplete, AzureDiscoveryMsg::DiscoveryComplete) => true,
            _ => false,
        }
    }
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
    QueueStatsUpdated(QueueStatsCache),
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
