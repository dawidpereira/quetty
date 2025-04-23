
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum ComponentId {
    Label,
}


#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
}

impl Default for Msg {
    fn default() -> Self {
        Self::AppClose
    }
}
