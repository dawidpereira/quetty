#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum ComponentId {
    Label,
    Messages,
    MessageDetails,
}

#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    ForceRedraw,
    Submit(Vec<String>),
    MessageActivity(MessageActivitMsg),
}

#[derive(Debug, PartialEq)]
pub enum MessageActivitMsg {
    RefreshMessageDetails(usize),
    EditMessage(usize),
    CancelEditMessage,
}

impl Default for Msg {
    fn default() -> Self {
        Self::AppClose
    }
}
