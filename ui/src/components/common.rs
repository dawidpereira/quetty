use server::model::MessageModel;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum ComponentId {
    Label,
    Messages,
    MessageDetails,
    QueuePicker,
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
}

#[derive(Debug, PartialEq)]
pub enum QueueActivityMsg {
    QueueSelected(String),
}

impl Default for Msg {
    fn default() -> Self {
        Self::AppClose
    }
}
