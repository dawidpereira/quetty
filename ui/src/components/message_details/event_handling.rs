use super::component::MessageDetails;
use crate::components::common::{MessageActivityMsg, Msg, PopupActivityMsg};
// use crate::components::message_details::navigation::NextPrevAction; // Not needed
use crate::config;
use crate::error::AppError;
use tuirealm::{
    NoUserEvent,
    event::{Event, Key, KeyEvent, KeyModifiers},
};

pub fn handle_event(details: &mut MessageDetails, ev: Event<NoUserEvent>) -> Option<Msg> {
    match ev {
        Event::Keyboard(KeyEvent {
            code: Key::Esc,
            modifiers: KeyModifiers::NONE,
        }) => {
            if details.is_editing {
                // In edit mode: restore original content and exit edit mode
                details.restore_original_content();
                return Some(Msg::MessageActivity(MessageActivityMsg::EditingModeStopped));
            } else {
                // Not in edit mode: exit to message list
                return Some(Msg::MessageActivity(MessageActivityMsg::CancelEditMessage));
            }
        }

        // Edit operations - Ctrl+s and Shift+Ctrl+s
        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }) if c == config::get_config_or_panic().keys().send_edited_message() => {
            if details.is_editing && details.is_dirty {
                // Validate content before sending
                let edited_content = details.get_edited_content();
                if let Err(validation_error) = details.validate_message_content(&edited_content) {
                    return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(
                        validation_error,
                    )));
                }

                // Send edited content as new message (keep original)
                return Some(Msg::MessageActivity(MessageActivityMsg::SendEditedMessage(
                    edited_content,
                )));
            } else if details.is_editing && !details.is_dirty {
                return Some(Msg::PopupActivity(PopupActivityMsg::ShowSuccess(
                    "ℹ️ No changes to send - content is unchanged".to_string(),
                )));
            }
        }

        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }) if c == config::get_config_or_panic().keys().replace_edited_message() => {
            if details.is_editing && details.is_dirty {
                if let Some(message) = &details.current_message {
                    // Validate content before replacing
                    let edited_content = details.get_edited_content();
                    if let Err(validation_error) = details.validate_message_content(&edited_content)
                    {
                        return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(
                            validation_error,
                        )));
                    }

                    // Replace original message with edited content
                    let message_id = message.id.clone();
                    return Some(Msg::MessageActivity(
                        MessageActivityMsg::ReplaceEditedMessage(edited_content, message_id.into()),
                    ));
                } else {
                    return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(
                        AppError::State("No message available for replacement".to_string()),
                    )));
                }
            } else if details.is_editing && !details.is_dirty {
                return Some(Msg::PopupActivity(PopupActivityMsg::ShowSuccess(
                    "ℹ️ No changes to replace - content is unchanged".to_string(),
                )));
            }
        }

        // Toggle edit mode with 'e' or 'i' key (similar to vim)
        Event::Keyboard(KeyEvent {
            code: Key::Char('e') | Key::Char('i'),
            modifiers: KeyModifiers::NONE,
        }) if !details.is_editing => {
            details.toggle_edit_mode();
            return Some(Msg::MessageActivity(MessageActivityMsg::EditingModeStarted));
        }

        // Editing keys (only when in edit mode)
        Event::Keyboard(KeyEvent {
            code: Key::Char(ch),
            modifiers: KeyModifiers::NONE,
        }) if details.is_editing => {
            details.insert_char(ch);
            return Some(Msg::ForceRedraw);
        }

        Event::Keyboard(KeyEvent {
            code: Key::Char(ch),
            modifiers: KeyModifiers::SHIFT,
        }) if details.is_editing => {
            details.insert_char(ch);
            return Some(Msg::ForceRedraw);
        }

        Event::Keyboard(KeyEvent {
            code: Key::Backspace,
            modifiers: KeyModifiers::NONE,
        }) if details.is_editing => {
            details.delete_char_backward();
            return Some(Msg::ForceRedraw);
        }

        Event::Keyboard(KeyEvent {
            code: Key::Delete,
            modifiers: KeyModifiers::NONE,
        }) if details.is_editing => {
            details.delete_char_forward();
            return Some(Msg::ForceRedraw);
        }

        Event::Keyboard(KeyEvent {
            code: Key::Enter,
            modifiers: KeyModifiers::NONE,
        }) if details.is_editing => {
            details.insert_newline();
            return Some(Msg::ForceRedraw);
        }

        Event::Keyboard(KeyEvent {
            code: Key::Up,
            modifiers: KeyModifiers::NONE,
        }) => {
            details.move_cursor_up();
        }

        Event::Keyboard(KeyEvent {
            code: Key::Down,
            modifiers: KeyModifiers::NONE,
        }) => {
            details.move_cursor_down(details.visible_lines);
        }

        Event::Keyboard(KeyEvent {
            code: Key::Left,
            modifiers: KeyModifiers::NONE,
        }) => {
            details.move_cursor_left();
        }

        Event::Keyboard(KeyEvent {
            code: Key::Right,
            modifiers: KeyModifiers::NONE,
        }) => {
            details.move_cursor_right();
        }

        Event::Keyboard(KeyEvent {
            code: Key::PageUp,
            modifiers: KeyModifiers::NONE,
        }) => {
            details.handle_page_navigation(true);
        }

        Event::Keyboard(KeyEvent {
            code: Key::PageDown,
            modifiers: KeyModifiers::NONE,
        }) => {
            details.handle_page_navigation(false);
        }

        Event::Keyboard(KeyEvent {
            code: Key::Home,
            modifiers: KeyModifiers::NONE,
        }) => {
            if details.is_editing {
                details.move_cursor_to_line_start();
            } else {
                details.move_cursor_to_top();
            }
        }

        Event::Keyboard(KeyEvent {
            code: Key::End,
            modifiers: KeyModifiers::NONE,
        }) => {
            if details.is_editing {
                details.move_cursor_to_line_end();
            } else {
                details.move_cursor_to_bottom();
            }
        }

        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }) if c == config::get_config_or_panic().keys().copy_message() => {
            // Copy message content to clipboard with Ctrl+configured_key
            match details.copy_to_clipboard() {
                Ok(()) => {
                    return Some(Msg::PopupActivity(PopupActivityMsg::ShowSuccess(
                        "📋 Message content copied to clipboard!".to_string(),
                    )));
                }
                Err(e) => {
                    log::error!("Failed to copy to clipboard: {}", e);
                    return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(
                        AppError::Component("Failed to copy message to clipboard".to_string()),
                    )));
                }
            }
        }

        Event::Keyboard(KeyEvent {
            code: Key::Char(c),
            modifiers: KeyModifiers::NONE,
        }) if c == config::get_config_or_panic().keys().yank_message() => {
            // Copy message content to clipboard with configured yank key
            match details.copy_to_clipboard() {
                Ok(()) => {
                    return Some(Msg::PopupActivity(PopupActivityMsg::ShowSuccess(
                        "📋 Message content yanked to clipboard!".to_string(),
                    )));
                }
                Err(e) => {
                    log::error!("Failed to copy to clipboard: {}", e);
                    return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(
                        AppError::Component("Failed to yank message to clipboard".to_string()),
                    )));
                }
            }
        }

        _ => {}
    }

    Some(Msg::ForceRedraw)
}
