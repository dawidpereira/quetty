use serde::Deserialize;

/// Key bindings configuration
#[derive(Debug, Deserialize, Default, Clone)]
pub struct KeyBindingsConfig {
    // Global keys
    key_quit: Option<char>,
    key_help: Option<char>,
    key_theme: Option<char>,
    key_config: Option<char>,
    key_refresh: Option<char>,

    // Navigation keys
    key_down: Option<char>,
    key_up: Option<char>,
    key_next_page: Option<char>,
    key_prev_page: Option<char>,
    key_alt_next_page: Option<char>,
    key_alt_prev_page: Option<char>,

    // Message actions
    key_resend_from_dlq: Option<char>,
    key_resend_and_delete_from_dlq: Option<char>,
    key_delete_message: Option<char>,
    key_alt_delete_message: Option<char>,

    // Message details actions
    key_copy_message: Option<char>,
    key_yank_message: Option<char>,
    key_send_edited_message: Option<char>,
    key_replace_edited_message: Option<char>,

    // Bulk selection keys
    key_toggle_selection: Option<char>,
    key_select_all_page: Option<char>,

    // Queue/Namespace selection
    key_queue_select: Option<char>,
    key_namespace_select: Option<char>,

    // Message composition keys
    key_toggle_dlq: Option<char>,
    key_compose_multiple: Option<char>,
    key_compose_single: Option<char>,

    // Page size selection
    key_page_size: Option<char>,

    // Confirmation keys
    key_confirm_yes: Option<char>,
    key_confirm_no: Option<char>,
}

impl KeyBindingsConfig {
    // Global keys
    pub fn quit(&self) -> char {
        self.key_quit.unwrap_or('q')
    }

    pub fn help(&self) -> char {
        self.key_help.unwrap_or('h')
    }

    pub fn theme(&self) -> char {
        self.key_theme.unwrap_or('t')
    }

    pub fn config(&self) -> char {
        self.key_config.unwrap_or('C')
    }

    pub fn refresh(&self) -> char {
        self.key_refresh.unwrap_or('r')
    }

    // Navigation keys
    pub fn down(&self) -> char {
        self.key_down.unwrap_or('j')
    }

    pub fn up(&self) -> char {
        self.key_up.unwrap_or('k')
    }

    pub fn next_page(&self) -> char {
        self.key_next_page.unwrap_or('n')
    }

    pub fn prev_page(&self) -> char {
        self.key_prev_page.unwrap_or('p')
    }

    pub fn alt_next_page(&self) -> char {
        self.key_alt_next_page.unwrap_or('N')
    }

    pub fn alt_prev_page(&self) -> char {
        self.key_alt_prev_page.unwrap_or('P')
    }

    // Message actions

    pub fn resend_from_dlq(&self) -> char {
        self.key_resend_from_dlq.unwrap_or('s')
    }

    pub fn resend_and_delete_from_dlq(&self) -> char {
        self.key_resend_and_delete_from_dlq.unwrap_or('R')
    }

    pub fn delete_message(&self) -> char {
        self.key_delete_message.unwrap_or('d')
    }

    pub fn alt_delete_message(&self) -> char {
        self.key_alt_delete_message.unwrap_or('D')
    }

    // Message details actions
    pub fn copy_message(&self) -> char {
        self.key_copy_message.unwrap_or('c')
    }

    pub fn yank_message(&self) -> char {
        self.key_yank_message.unwrap_or('y')
    }

    pub fn send_edited_message(&self) -> char {
        self.key_send_edited_message.unwrap_or('s')
    }

    pub fn replace_edited_message(&self) -> char {
        self.key_replace_edited_message.unwrap_or('r')
    }

    // Bulk selection keys
    pub fn toggle_selection(&self) -> char {
        self.key_toggle_selection.unwrap_or(' ')
    }

    pub fn select_all_page(&self) -> char {
        self.key_select_all_page.unwrap_or('a')
    }

    // Queue/Namespace selection
    pub fn queue_select(&self) -> char {
        self.key_queue_select.unwrap_or('o')
    }

    pub fn namespace_select(&self) -> char {
        self.key_namespace_select.unwrap_or('o')
    }

    // Message composition keys
    pub fn toggle_dlq(&self) -> char {
        self.key_toggle_dlq.unwrap_or('d')
    }

    pub fn compose_multiple(&self) -> char {
        self.key_compose_multiple.unwrap_or('m')
    }

    pub fn compose_single(&self) -> char {
        self.key_compose_single.unwrap_or('M')
    }

    // Page size selection
    pub fn page_size(&self) -> char {
        self.key_page_size.unwrap_or('z')
    }

    // Confirmation keys
    pub fn confirm_yes(&self) -> char {
        self.key_confirm_yes.unwrap_or('y')
    }

    pub fn confirm_no(&self) -> char {
        self.key_confirm_no.unwrap_or('n')
    }
}
