use crate::app::model::Model;
use crate::components::common::{LoadingActivityMsg, Msg, NamespaceActivityMsg};
use crate::config;
use crate::config::CONFIG;
use crate::error::{AppError, AppResult};
use crate::theme::ThemeManager;
use server::service_bus_manager::ServiceBusManager;
use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Key, KeyEvent};
use tuirealm::props::{Alignment, Style, TextModifiers};
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::widgets::{List, ListItem};
use tuirealm::terminal::TerminalAdapter;
use tuirealm::{
    AttrValue, Attribute, Component, Event, Frame, MockComponent, NoUserEvent, State, StateValue,
};

const CMD_RESULT_NAMESPACE_SELECTED: &str = "NamespaceSelected";

pub struct NamespacePicker {
    namespaces: Vec<String>,
    selected: usize,
}

impl NamespacePicker {
    pub fn new(namespaces: Option<Vec<String>>) -> Self {
        Self {
            namespaces: namespaces.unwrap_or_default(),
            selected: 0,
        }
    }
}

impl MockComponent for NamespacePicker {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let theme = ThemeManager::global();

        // Calculate consistent width for namespace icons and names
        let formatted_items: Vec<String> = self
            .namespaces
            .iter()
            .map(|ns| format!("🏢 {}", ns))
            .collect();

        // Find maximum width needed for proper alignment
        let max_width = formatted_items
            .iter()
            .map(|item| item.len())
            .max()
            .unwrap_or(30);
        let padding_width = max_width + 4;

        let items: Vec<ListItem> = self
            .namespaces
            .iter()
            .enumerate()
            .map(|(i, ns)| {
                let namespace_text =
                    format!("{:width$}", format!("🏢 {}", ns), width = padding_width);
                let mut item = ListItem::new(namespace_text);
                if i == self.selected {
                    item = item.style(
                        Style::default()
                            .fg(theme.namespace_list_item())
                            .bg(theme.surface())
                            .add_modifier(TextModifiers::BOLD),
                    );
                } else {
                    item = item.style(Style::default().fg(theme.namespace_list_item()));
                }
                item
            })
            .collect();
        let list = List::new(items)
            .block(
                tuirealm::ratatui::widgets::Block::default()
                    .borders(tuirealm::ratatui::widgets::Borders::ALL)
                    .border_style(Style::default().fg(theme.primary_accent()))
                    .title("  🌐 Select a Namespace  ")
                    .title_alignment(Alignment::Center)
                    .title_style(
                        Style::default()
                            .fg(theme.title_accent())
                            .add_modifier(TextModifiers::BOLD),
                    ),
            )
            .highlight_style(
                Style::default()
                    .fg(theme.namespace_list_item())
                    .bg(theme.surface())
                    .add_modifier(TextModifiers::BOLD),
            )
            .highlight_symbol("▶ ");
        frame.render_widget(list, area);
    }
    fn query(&self, _attr: Attribute) -> Option<AttrValue> {
        None
    }
    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {}
    fn state(&self) -> tuirealm::State {
        if let Some(ns) = self.namespaces.get(self.selected) {
            tuirealm::State::One(tuirealm::StateValue::String(ns.clone()))
        } else {
            tuirealm::State::None
        }
    }
    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for NamespacePicker {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd_result = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => {
                if self.selected + 1 < self.namespaces.len() {
                    self.selected += 1;
                }
                CmdResult::Changed(State::One(StateValue::Usize(self.selected)))
            }
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                CmdResult::Changed(State::One(StateValue::Usize(self.selected)))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => {
                if let Some(ns) = self.namespaces.get(self.selected).cloned() {
                    CmdResult::Custom(
                        CMD_RESULT_NAMESPACE_SELECTED,
                        State::One(StateValue::String(ns)),
                    )
                } else {
                    CmdResult::None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c), ..
            }) => {
                let keys = config::CONFIG.keys();
                if c == keys.down() {
                    if self.selected + 1 < self.namespaces.len() {
                        self.selected += 1;
                    }
                    CmdResult::Changed(State::One(StateValue::Usize(self.selected)))
                } else if c == keys.up() {
                    if self.selected > 0 {
                        self.selected -= 1;
                    }
                    CmdResult::Changed(State::One(StateValue::Usize(self.selected)))
                } else if c == keys.namespace_select() {
                    if let Some(ns) = self.namespaces.get(self.selected).cloned() {
                        CmdResult::Custom(
                            CMD_RESULT_NAMESPACE_SELECTED,
                            State::One(StateValue::String(ns)),
                        )
                    } else {
                        CmdResult::None
                    }
                } else {
                    CmdResult::None
                }
            }
            _ => CmdResult::None,
        };

        match cmd_result {
            CmdResult::Custom(CMD_RESULT_NAMESPACE_SELECTED, state) => {
                if let State::One(StateValue::String(_)) = state {
                    Some(Msg::NamespaceActivity(
                        NamespaceActivityMsg::NamespaceSelected,
                    ))
                } else {
                    None
                }
            }
            _ => Some(Msg::ForceRedraw),
        }
    }
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn load_namespaces(&mut self) -> AppResult<()> {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Show loading indicator
        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Start(
            "Loading namespaces...".to_string(),
        ))) {
            log::error!("Failed to send loading start message: {}", e);
        }

        let tx_to_main_err = tx_to_main.clone();
        taskpool.execute(async move {
            let result = async {
                log::debug!("Requesting namespaces from Azure AD");

                // Send an update that we're requesting namespaces
                if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Update(
                    "Connecting to Azure AD...".to_string(),
                ))) {
                    log::error!("Failed to send loading update message: {}", e);
                }

                let namespaces = ServiceBusManager::list_namespaces_azure_ad(CONFIG.azure_ad())
                    .await
                    .map_err(|e| {
                        log::error!("Failed to list namespaces: {}", e);
                        AppError::ServiceBus(e.to_string())
                    })?;

                // Send an update that we've received namespaces
                if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Update(
                    "Processing namespaces...".to_string(),
                ))) {
                    log::error!("Failed to send loading update message: {}", e);
                }

                log::info!("Loaded {} namespaces", namespaces.len());

                // Stop loading indicator
                if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
                    log::error!("Failed to send loading stop message: {}", e);
                }

                // Send loaded namespaces
                tx_to_main
                    .send(Msg::NamespaceActivity(
                        NamespaceActivityMsg::NamespacesLoaded(namespaces),
                    ))
                    .map_err(|e| {
                        log::error!("Failed to send namespaces loaded message: {}", e);
                        AppError::Component(e.to_string())
                    })?;

                Ok::<(), AppError>(())
            }
            .await;
            if let Err(e) = result {
                log::error!("Error in namespace loading task: {}", e);

                // Stop loading indicator even if there was an error
                if let Err(err) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
                    log::error!("Failed to send loading stop message: {}", err);
                }

                // Send error message
                let _ = tx_to_main_err.send(Msg::Error(e));
            }
        });

        Ok(())
    }
}
