use super::component::{
    CMD_RESULT_MESSAGE_PREVIEW, CMD_RESULT_MESSAGE_SELECTED, CMD_RESULT_QUEUE_UNSELECTED, Messages,
};
use super::selection::create_toggle_message_selection;
use crate::components::common::{MessageActivityMsg, Msg, QueueActivityMsg, QueueType};
use crate::config;
use tuirealm::command::CmdResult;
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::{Event, MockComponent, NoUserEvent, State, StateValue};

pub fn handle_event(messages: &mut Messages, ev: Event<NoUserEvent>) -> Option<Msg> {
    let cmd_result = match ev {
        // Bulk selection key bindings
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::CONFIG.keys().toggle_selection() => {
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
        }) if c == config::CONFIG.keys().select_all_page() => {
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

        // Enhanced existing operations for bulk mode - context-aware send/resend
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }) if c == config::CONFIG.keys().send_to_dlq() => {
            // Context-aware operation based on current queue type
            if let Some(pagination_info) = messages.pagination_info() {
                match pagination_info.queue_type {
                    QueueType::Main => {
                        // In main queue: send to DLQ
                        return Some(Msg::MessageActivity(
                            MessageActivityMsg::BulkSendSelectedToDLQ,
                        ));
                    }
                    QueueType::DeadLetter => {
                        // In DLQ: resend to main queue (keep in DLQ)
                        return Some(Msg::MessageActivity(
                            MessageActivityMsg::BulkResendSelectedFromDLQ(false),
                        ));
                    }
                }
            } else {
                // Fallback to send to DLQ if no pagination info available
                return Some(Msg::MessageActivity(
                    MessageActivityMsg::BulkSendSelectedToDLQ,
                ));
            }
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::CONFIG.keys().send_to_dlq() => {
            // Context-aware operation based on current queue type
            if let Some(pagination_info) = messages.pagination_info() {
                match pagination_info.queue_type {
                    QueueType::Main => {
                        // In main queue: send to DLQ
                        return Some(Msg::MessageActivity(
                            MessageActivityMsg::BulkSendSelectedToDLQ,
                        ));
                    }
                    QueueType::DeadLetter => {
                        // In DLQ: resend to main queue (keep in DLQ)
                        return Some(Msg::MessageActivity(
                            MessageActivityMsg::BulkResendSelectedFromDLQ(false),
                        ));
                    }
                }
            } else {
                // Fallback to send to DLQ if no pagination info available
                return Some(Msg::MessageActivity(
                    MessageActivityMsg::BulkSendSelectedToDLQ,
                ));
            }
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::CONFIG.keys().resend_from_dlq() => {
            // Resend only (without deleting from DLQ)
            return Some(Msg::MessageActivity(
                MessageActivityMsg::BulkResendSelectedFromDLQ(false),
            ));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::SHIFT,
        }) if c == config::CONFIG.keys().resend_and_delete_from_dlq() => {
            // Resend and delete from DLQ
            return Some(Msg::MessageActivity(
                MessageActivityMsg::BulkResendSelectedFromDLQ(true),
            ));
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
        }) if c == config::CONFIG.keys().alt_delete_message() => {
            // Bulk delete with Ctrl+X
            return Some(Msg::MessageActivity(MessageActivityMsg::BulkDeleteSelected));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::CONFIG.keys().delete_message() => {
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
        }) if c == config::CONFIG.keys().down() => {
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
        }) if c == config::CONFIG.keys().up() => {
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
        }) if c == config::CONFIG.keys().next_page() => {
            return Some(Msg::MessageActivity(MessageActivityMsg::NextPage));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::CONFIG.keys().alt_next_page() => {
            return Some(Msg::MessageActivity(MessageActivityMsg::NextPage));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::CONFIG.keys().prev_page() => {
            return Some(Msg::MessageActivity(MessageActivityMsg::PreviousPage));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::CONFIG.keys().alt_prev_page() => {
            return Some(Msg::MessageActivity(MessageActivityMsg::PreviousPage));
        }

        // Global navigation
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::CONFIG.keys().help() => {
            return Some(Msg::ToggleHelpScreen);
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::CONFIG.keys().quit() => return Some(Msg::AppClose),
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }) if c == config::CONFIG.keys().quit() => return Some(Msg::AppClose),
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::CONFIG.keys().quit() => {
            return Some(Msg::QueueActivity(QueueActivityMsg::QueueUnselected));
        }

        // Queue toggle
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::CONFIG.keys().toggle_dlq() => {
            return Some(Msg::QueueActivity(QueueActivityMsg::ToggleDeadLetterQueue));
        }

        // Message composition
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::CONFIG.keys().compose_multiple() => {
            return Some(Msg::MessageActivity(
                MessageActivityMsg::SetMessageRepeatCount,
            ));
        }
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }) if c == config::CONFIG.keys().compose_single() => {
            return Some(Msg::MessageActivity(MessageActivityMsg::ComposeNewMessage));
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

