use crate::config::keys::KeyBindingsConfig;

/// Represents a single keyboard shortcut with its description
#[derive(Debug, Clone)]
pub struct Shortcut {
    pub keys: Vec<String>,
    pub description: String,
}

impl Shortcut {
    pub fn new(keys: Vec<String>, description: &str) -> Self {
        Self {
            keys,
            description: description.to_string(),
        }
    }

    /// Create a shortcut with a single key
    pub fn single(key: String, description: &str) -> Self {
        Self::new(vec![key], description)
    }

    /// Create a shortcut with multiple key alternatives
    pub fn multiple(keys: Vec<String>, description: &str) -> Self {
        Self::new(keys, description)
    }
}

/// Represents a section of help content
#[derive(Debug, Clone)]
pub struct HelpSection {
    pub title: String,
    pub icon: String,
    pub shortcuts: Vec<Shortcut>,
}

impl HelpSection {
    pub fn new(title: &str, icon: &str) -> Self {
        Self {
            title: title.to_string(),
            icon: icon.to_string(),
            shortcuts: Vec::new(),
        }
    }

    pub fn add_single_key(mut self, key: String, description: &str) -> Self {
        self.shortcuts.push(Shortcut::single(key, description));
        self
    }

    pub fn add_multiple_keys(mut self, keys: Vec<String>, description: &str) -> Self {
        self.shortcuts.push(Shortcut::multiple(keys, description));
        self
    }
}

/// Contains all help content organized by sections
#[derive(Debug, Clone)]
pub struct HelpContent {
    pub header_message: String,
    pub warning_message: String,
    pub sections: Vec<HelpSection>,
}

impl HelpContent {
    /// Generate help content from configuration
    pub fn from_config(keys: &KeyBindingsConfig) -> Self {
        let sections = vec![
            // LEFT COLUMN - Global Actions Section
            HelpSection::new("GLOBAL ACTIONS", "🌐")
                .add_single_key(format!("[{}]", keys.quit()), "Quit application")
                .add_single_key(format!("[{}]", keys.help()), "Toggle this help screen")
                .add_single_key(format!("[{}]", keys.theme()), "Open theme picker")
                .add_single_key("[Esc]".to_string(), "Go back / Cancel operation"),
            // Navigation Section
            HelpSection::new("NAVIGATION", "🧭")
                .add_multiple_keys(vec![format!("[↑]"), format!("[{}]", keys.up())], "Move up")
                .add_multiple_keys(
                    vec![format!("[↓]"), format!("[{}]", keys.down())],
                    "Move down",
                )
                .add_multiple_keys(
                    vec![format!("[Enter]"), format!("[{}]", keys.queue_select())],
                    "Select / Open item",
                )
                .add_single_key("[PgUp] [PgDn]".to_string(), "Scroll page up/down"),
            // Queue & Message Management Section
            HelpSection::new("QUEUE & MESSAGE MANAGEMENT", "📋")
                .add_multiple_keys(
                    vec![
                        format!("[{}]", keys.next_page()),
                        format!("[{}]", keys.alt_next_page()),
                    ],
                    "Next page",
                )
                .add_multiple_keys(
                    vec![
                        format!("[{}]", keys.prev_page()),
                        format!("[{}]", keys.alt_prev_page()),
                    ],
                    "Previous page",
                )
                .add_single_key(
                    format!("[{}]", keys.toggle_dlq()),
                    "Toggle Main ↔ Dead Letter Queue",
                )
                .add_single_key("[Enter]".to_string(), "View message details"),
            // Message Composition Section
            HelpSection::new("MESSAGE COMPOSITION", "✍️")
                .add_single_key(
                    format!("[{}]", keys.compose_multiple()),
                    "Compose multiple messages",
                )
                .add_single_key(
                    format!("[Ctrl+{}]", keys.compose_single()),
                    "Compose single message",
                ),
            // Confirmations Section
            HelpSection::new("CONFIRMATIONS", "✅")
                .add_single_key(format!("[{}]", keys.confirm_yes()), "Confirm Yes")
                .add_single_key(format!("[{}]", keys.confirm_no()), "Confirm No"),
            // RIGHT COLUMN - Bulk Selection Mode
            HelpSection::new("BULK SELECTION MODE", "📦")
                .add_single_key("[ ]".to_string(), "Toggle selection for current message")
                .add_single_key(
                    "[Ctrl+a]".to_string(),
                    "Select all messages on current page",
                )
                .add_single_key(
                    "[Ctrl+Shift+A]".to_string(),
                    "Select all loaded messages (all pages)",
                )
                .add_single_key("[Esc]".to_string(), "Clear selections / Exit bulk mode"),
            // Message Operations Section
            HelpSection::new("MESSAGE OPERATIONS", "⚡")
                .add_multiple_keys(
                    vec![format!("[x]"), format!("[Ctrl+x]")],
                    "Delete message(s) with confirmation",
                )
                .add_single_key("[S]".to_string(), "Send message(s) to DLQ (⚠️ DEV)")
                .add_single_key(
                    format!("[{}]", keys.resend_from_dlq()),
                    "Resend from DLQ to main queue (keep in DLQ)",
                )
                .add_single_key(
                    format!("[{}]", keys.resend_and_delete_from_dlq()),
                    "Resend and delete from DLQ (⚠️ DEV)",
                ),
            // Add note as a special section
            HelpSection::new("Note", "💡")
                .add_single_key(
                    "Operations work on selected messages in bulk mode,".to_string(),
                    "",
                )
                .add_single_key(
                    "or on current message when no selections exist.".to_string(),
                    "",
                ),
            // Message Details View Section
            HelpSection::new("MESSAGE DETAILS VIEW", "🔍")
                .add_single_key("[←] [→]".to_string(), "Move cursor left/right")
                .add_multiple_keys(
                    vec![
                        format!("[↑] [↓]"),
                        format!("[{}] [{}]", keys.up(), keys.down()),
                    ],
                    "Scroll content up/down",
                )
                .add_single_key("[PgUp] [PgDn]".to_string(), "Scroll content page up/down")
                .add_multiple_keys(
                    vec![
                        format!("[{}]", keys.yank_message()),
                        format!("[Ctrl+{}]", keys.copy_message()),
                    ],
                    "Copy message content to clipboard",
                )
                .add_single_key(
                    "[e] [i]".to_string(),
                    "Enter edit mode to modify message content",
                )
                .add_single_key(
                    "[Ctrl+s]".to_string(),
                    "Send edited content as new message (keep original)",
                )
                .add_single_key(
                    "[Ctrl+r]".to_string(),
                    "Replace original message with edited content",
                )
                .add_single_key("[Esc]".to_string(), "Return to message list"),
        ];

        Self {
            header_message: format!("Press [Esc] or [{}] to close this help screen", keys.help()),
            warning_message: "⚠️  DLQ operations are in development - use with caution".to_string(),
            sections,
        }
    }
}
