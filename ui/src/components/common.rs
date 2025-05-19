use server::consumer::Consumer;
use server::model::MessageModel;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum ComponentId {
    Label,
    Messages,
    MessageDetails,
    QueuePicker,
    GlobalKeyWatcher,
}

#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    ForceRedraw,
    Submit(Vec<String>),
    MessageActivity(MessageActivityMsg),
    QueueActivity(QueueActivityMsg),
}

#[derive(Debug, PartialEq)]
pub enum MessageActivityMsg {
    RefreshMessageDetails(usize),
    EditMessage(usize),
    CancelEditMessage,
    MessagesLoaded(Vec<MessageModel>),
    ConsumerCreated(Consumer),
}

#[derive(Debug, PartialEq)]
pub enum QueueActivityMsg {
    QueueSelected(String),
    QueueUnfocused,
}

impl Default for Msg {
    fn default() -> Self {
        Self::AppClose
    }
}
