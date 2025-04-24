
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum ComponentId {
    Label,
    Messages
}


#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    ForceRedraw,
}


impl Default for Msg {
    fn default() -> Self {
        Self::AppClose
    }
}
