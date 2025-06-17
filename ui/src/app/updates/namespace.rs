use crate::app::model::Model;
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
                    self.error_reporter
                        .report_simple(e, "NamespaceHandler", "update_namespace");
                    return None;
                }
                None
            }
            NamespaceActivityMsg::NamespaceSelected => self.handle_namespace_selection(),
            NamespaceActivityMsg::NamespaceUnselected => {
                // Clear selected namespace
                self.selected_namespace = None;

                self.load_namespaces();
                None
            }
        }
    }

    /// Handle namespace selection by storing the selected namespace and loading queues
    fn handle_namespace_selection(&mut self) -> Option<Msg> {
        // Store the currently selected namespace from the namespace picker component
        if let Ok(State::One(tuirealm::StateValue::String(namespace))) =
            self.app.state(&ComponentId::NamespacePicker)
        {
            log::info!("Selected namespace: {}", namespace);
            self.selected_namespace = Some(namespace);
        }

        self.load_queues();
        None
    }
}
