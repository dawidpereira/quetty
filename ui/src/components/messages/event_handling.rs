use super::component::{
    CMD_RESULT_MESSAGE_PREVIEW, CMD_RESULT_MESSAGE_SELECTED, CMD_RESULT_QUEUE_UNSELECTED, Messages,
};
use super::selection::create_toggle_message_selection;
use crate::components::common::{MessageActivityMsg, Msg, QueueActivityMsg};
use crate::config;
use tuirealm::command::CmdResult;
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::{Event, MockComponent, NoUserEvent, State, StateValue};
use server::service_bus_manager::QueueType;

pub fn handle_event(messages: &mut Messages, ev: Event<NoUserEvent>) -> Option<Msg> {
    let cmd_result = match ev {
        // Bulk selection key bindings
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::get_config_or_panic().keys().toggle_selection() => {
            // Toggle selection for current message
            let index = match messages.state() {
                State::One(StateValue::Usize(index)) => index,
                _ => 0,
            };

            return Some(create_toggle_message_selection(index));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }) if c == config::get_config_or_panic().keys().select_all_page() => {
            // Select all messages on current page
            return Some(Msg::MessageActivity(
                MessageActivityMsg::SelectAllCurrentPage,
            ));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char('A'),
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        }) => {
            // Select all loaded messages across all pages
            return Some(Msg::MessageActivity(
                MessageActivityMsg::SelectAllLoadedMessages,
            ));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Esc,
            modifiers: KeyModifiers::NONE,
        }) => {
            // In bulk mode, clear selections. Otherwise, go back
            // We'll let the handler decide based on current state
            return Some(Msg::MessageActivity(MessageActivityMsg::ClearAllSelections));
        }

        Event::Keyboard(KeyEvent {
            code: Key::Delete,
            modifiers: KeyModifiers::NONE,
        }) => {
            // Check if we should do bulk operation or single message operation
            return Some(Msg::MessageActivity(MessageActivityMsg::BulkDeleteSelected));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }) if c == config::get_config_or_panic().keys().alt_delete_message() => {
            // Bulk delete with Ctrl+X
            return Some(Msg::MessageActivity(MessageActivityMsg::BulkDeleteSelected));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::get_config_or_panic().keys().delete_message() => {
            // Single delete with X
            return Some(Msg::MessageActivity(MessageActivityMsg::BulkDeleteSelected));
        }

        // Navigation keys
        Event::Keyboard(KeyEvent {
            code: Key::Down,
            modifiers: KeyModifiers::NONE,
        }) => {
            messages.move_down();
            CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, messages.state())
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::get_config_or_panic().keys().down() => {
            messages.move_down();
            CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, messages.state())
        }
        Event::Keyboard(KeyEvent {
            code: Key::Up,
            modifiers: KeyModifiers::NONE,
        }) => {
            messages.move_up();
            CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, messages.state())
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::get_config_or_panic().keys().up() => {
            messages.move_up();
            CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, messages.state())
        }
        Event::Keyboard(KeyEvent {
            code: Key::PageDown,
            modifiers: KeyModifiers::NONE,
        }) => {
            messages.page_down();
            CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, messages.state())
        }
        Event::Keyboard(KeyEvent {
            code: Key::PageUp,
            modifiers: KeyModifiers::NONE,
        }) => {
            messages.page_up();
            CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, messages.state())
        }
        Event::Keyboard(KeyEvent {
            code: Key::Enter,
            modifiers: KeyModifiers::NONE,
        }) => CmdResult::Custom(CMD_RESULT_MESSAGE_SELECTED, messages.state()),

        // Pagination
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::get_config_or_panic().keys().next_page() => {
            return Some(Msg::MessageActivity(MessageActivityMsg::NextPage));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::get_config_or_panic().keys().alt_next_page() => {
            return Some(Msg::MessageActivity(MessageActivityMsg::NextPage));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::get_config_or_panic().keys().prev_page() => {
            return Some(Msg::MessageActivity(MessageActivityMsg::PreviousPage));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::get_config_or_panic().keys().alt_prev_page() => {
            return Some(Msg::MessageActivity(MessageActivityMsg::PreviousPage));
        }

        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::get_config_or_panic().keys().quit() => return Some(Msg::AppClose),
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }) if c == config::get_config_or_panic().keys().quit() => return Some(Msg::AppClose),

        // Queue toggle
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::get_config_or_panic().keys().toggle_dlq() => {
            return Some(Msg::QueueActivity(QueueActivityMsg::ToggleDeadLetterQueue));
        }

        // Message composition
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::get_config_or_panic().keys().compose_multiple() => {
            // Check if we're in DLQ - composition not allowed
            if let Some(pagination_info) = messages.pagination_info() {
                match pagination_info.queue_type {
                    QueueType::DeadLetter => {
                        return Some(Msg::ShowError("❌ Cannot compose new messages from Dead Letter Queue.\n\n💡 Switch to Main queue first using 'D' key.\n📖 Composition is only available in Main queues.".to_string()));
                    }
                    QueueType::Main => {
                        return Some(Msg::MessageActivity(
                            MessageActivityMsg::SetMessageRepeatCount,
                        ));
                    }
                }
            } else {
                return Some(Msg::ShowError("❌ Unable to determine queue type. Please try switching queues.".to_string()));
            }
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }) if c == config::get_config_or_panic().keys().compose_single() => {
            // Check if we're in DLQ - composition not allowed
            if let Some(pagination_info) = messages.pagination_info() {
                match pagination_info.queue_type {
                    QueueType::DeadLetter => {
                        return Some(Msg::ShowError("❌ Cannot compose new messages from Dead Letter Queue.\n\n💡 Switch to Main queue first using 'D' key.\n📖 Composition is only available in Main queues.".to_string()));
                    }
                    QueueType::Main => {
                        return Some(Msg::MessageActivity(MessageActivityMsg::ComposeNewMessage));
                    }
                }
            } else {
                return Some(Msg::ShowError("❌ Unable to determine queue type. Please try switching queues.".to_string()));
            }
        }

        // Context-aware bulk send operations
        Event::Keyboard(KeyEvent {
            code: Key::Char('s'),
            modifiers: KeyModifiers::NONE,
        }) => {
            // Get pagination info to determine queue type
            if let Some(pagination_info) = messages.pagination_info() {
                                  match pagination_info.queue_type {
                     QueueType::Main => {
                         // Main queue: 's' key is not supported (no copy to DLQ)
                         return Some(Msg::ShowError("❌ Copy to DLQ is not supported by Azure Service Bus.\n\n💡 Use 'S' (Shift+s) to move messages to DLQ instead.\n📖 In Main Queue: 'S' = Move to DLQ".to_string()));
                     }
                     QueueType::DeadLetter => {
                        // DLQ: 's' = resend to main without deleting from DLQ
                        return Some(Msg::MessageActivity(
                            MessageActivityMsg::BulkResendSelectedFromDLQ(false),
                        ));
                    }
                }
            } else {
                return Some(Msg::ShowError("❌ Unable to determine queue type. Please try switching queues.".to_string()));
            }
        }

        Event::Keyboard(KeyEvent {
            code: Key::Char('S'),
            modifiers: KeyModifiers::SHIFT,
        }) => {
            // Get pagination info to determine queue type
            if let Some(pagination_info) = messages.pagination_info() {
                                  match pagination_info.queue_type {
                     QueueType::Main => {
                         // Main queue: 'S' = move to DLQ (with deletion)
                         return Some(Msg::MessageActivity(
                             MessageActivityMsg::BulkSendSelectedToDLQWithDelete,
                         ));
                     }
                     QueueType::DeadLetter => {
                        // DLQ: 'S' = resend to main with deletion from DLQ
                        return Some(Msg::MessageActivity(
                            MessageActivityMsg::BulkResendSelectedFromDLQ(true),
                        ));
                    }
                }
            } else {
                return Some(Msg::ShowError("❌ Unable to determine queue type. Please try switching queues.".to_string()));
            }
        }

        _ => CmdResult::None,
    };

    match cmd_result {
        CmdResult::Custom(CMD_RESULT_MESSAGE_SELECTED, State::One(StateValue::Usize(index))) => {
            Some(Msg::MessageActivity(MessageActivityMsg::EditMessage(index)))
        }
        CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, State::One(StateValue::Usize(index))) => {
            Some(Msg::MessageActivity(
                MessageActivityMsg::PreviewMessageDetails(index),
            ))
        }
        CmdResult::Custom(CMD_RESULT_QUEUE_UNSELECTED, _) => {
            Some(Msg::QueueActivity(QueueActivityMsg::QueueUnselected))
        }
        _ => Some(Msg::ForceRedraw),
    }
}
