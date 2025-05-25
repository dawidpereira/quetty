use crate::app::model::{AppState, Model};
use crate::components::common::{ComponentId, Msg, NamespaceActivityMsg};
use tuirealm::State;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_namespace(&mut self, msg: NamespaceActivityMsg) -> Option<Msg> {
        match msg {
            NamespaceActivityMsg::NamespacesLoaded(namespace) => {
                if let Err(e) = self.remount_namespace_picker(Some(namespace)) {
                    return Some(Msg::Error(e));
                }
                self.app_state = AppState::NamespacePicker;
                None
            }
            NamespaceActivityMsg::NamespaceSelected => {
                // Store the currently selected namespace from the namespace picker component
                if let Ok(State::One(tuirealm::StateValue::String(namespace))) =
                    self.app.state(&ComponentId::NamespacePicker)
                {
                    log::info!("Selected namespace: {}", namespace);
                    self.selected_namespace = Some(namespace);
                }

                if let Err(e) = self.load_queues() {
                    return Some(Msg::Error(e));
                }
                None
            }
            NamespaceActivityMsg::NamespaceUnselected => {
                // Clear selected namespace
                self.selected_namespace = None;

                if let Err(e) = self.load_namespaces() {
                    return Some(Msg::Error(e));
                }
                None
            }
        }
    }
}

