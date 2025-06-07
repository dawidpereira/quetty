use crate::theme::types::ThemeConfig;
use config::{Config, Environment, File};
use lazy_static::lazy_static;
use serde::Deserialize;
use server::bulk_operations::BatchConfig;
use server::service_bus_manager::AzureAdConfig;
use std::time::Duration;

lazy_static! {
    pub static ref CONFIG: AppConfig = {
        dotenv::dotenv().ok();
        let env_source = Environment::default().separator("__");
        let file_source = File::with_name("config.toml");

        let config = Config::builder()
            .add_source(file_source)
            .add_source(env_source)
            .build()
            .expect("Failed to load configuration");

        config
            .try_deserialize::<AppConfig>()
            .expect("Failed to deserialize configuration")
    };
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    max_messages: Option<u32>,
    crossterm_input_listener_interval_ms: Option<u64>,
    crossterm_input_listener_retries: Option<usize>,
    poll_timeout_ms: Option<u64>,
    tick_interval_millis: Option<u64>,
    #[serde(flatten)]
    dlq: DLQConfig,
    #[serde(flatten)]
    batch: BatchConfig,
    #[serde(flatten)]
    ui: UIConfig,
    keys: KeyBindingsConfig,
    servicebus: ServicebusConfig,
    azure_ad: AzureAdConfig,
    logging: LoggingConfig,
    theme: Option<ThemeConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    level: Option<String>,
    file: Option<String>,
}

/// Configuration for UI elements
#[derive(Debug, Clone, Deserialize)]
pub struct UIConfig {
    /// Duration between animation frames for loading indicators (default: 100ms)
    ui_loading_frame_duration_ms: Option<u64>,
}

/// Configuration for Dead Letter Queue (DLQ) operations
#[derive(Debug, Clone, Deserialize)]
pub struct DLQConfig {
    /// Timeout for receiving messages from DLQ (default: 10 seconds)
    dlq_receive_timeout_secs: Option<u64>,
    /// Maximum attempts to find a message in DLQ (default: 10)
    dlq_max_attempts: Option<usize>,
    /// Hard cap for receive timeouts (default: 10 seconds)
    dlq_receive_timeout_cap_secs: Option<u64>,
    /// Delay between retry attempts when no messages found (default: 500ms)
    dlq_retry_delay_ms: Option<u64>,
    /// Batch size for receiving messages in DLQ operations (default: 10)
    dlq_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServicebusConfig {
    connection_string: Option<String>,
}

/// Configuration for key bindings
#[derive(Debug, Clone, Deserialize)]
pub struct KeyBindingsConfig {
    // Global keys
    key_quit: Option<char>,
    key_help: Option<char>,
    key_theme: Option<char>,

    // Navigation keys
    key_down: Option<char>,
    key_up: Option<char>,
    key_next_page: Option<char>,
    key_prev_page: Option<char>,
    key_alt_next_page: Option<char>,
    key_alt_prev_page: Option<char>,

    // Message actions
    key_send_to_dlq: Option<char>,
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

    // Confirmation keys
    key_confirm_yes: Option<char>,
    key_confirm_no: Option<char>,
}

impl AppConfig {
    pub fn max_messages(&self) -> u32 {
        self.max_messages.unwrap_or(10)
    }

    pub fn crossterm_input_listener_interval(&self) -> Duration {
        Duration::from_millis(self.crossterm_input_listener_interval_ms.unwrap_or(20))
    }
    pub fn crossterm_input_listener_retries(&self) -> usize {
        self.crossterm_input_listener_retries.unwrap_or(5)
    }
    pub fn poll_timeout(&self) -> Duration {
        Duration::from_millis(self.poll_timeout_ms.unwrap_or(10))
    }
    pub fn tick_interval(&self) -> Duration {
        Duration::from_millis(self.tick_interval_millis.unwrap_or(250))
    }
    pub fn dlq(&self) -> &DLQConfig {
        &self.dlq
    }
    pub fn batch(&self) -> &BatchConfig {
        &self.batch
    }
    pub fn ui(&self) -> &UIConfig {
        &self.ui
    }
    pub fn keys(&self) -> &KeyBindingsConfig {
        &self.keys
    }
    pub fn servicebus(&self) -> &ServicebusConfig {
        &self.servicebus
    }
    pub fn azure_ad(&self) -> &AzureAdConfig {
        &self.azure_ad
    }
    pub fn logging(&self) -> &LoggingConfig {
        &self.logging
    }
    pub fn theme(&self) -> ThemeConfig {
        self.theme.clone().unwrap_or_default()
    }
}

impl ServicebusConfig {
    pub fn connection_string(&self) -> &str {
        self.connection_string.as_deref()
            .expect("SERVICEBUS_CONNECTION_STRING is required but not found in configuration or environment variables. Please set this value in .env file or environment.")
    }
}

impl DLQConfig {
    /// Get the timeout for receiving messages from DLQ
    pub fn receive_timeout_secs(&self) -> u64 {
        self.dlq_receive_timeout_secs.unwrap_or(10)
    }

    /// Get the maximum attempts to find a message in DLQ
    pub fn max_attempts(&self) -> usize {
        self.dlq_max_attempts.unwrap_or(10)
    }

    /// Get the hard cap for receive timeouts
    pub fn receive_timeout_cap_secs(&self) -> u64 {
        self.dlq_receive_timeout_cap_secs.unwrap_or(10)
    }

    /// Get the delay between retry attempts when no messages found
    pub fn retry_delay_ms(&self) -> u64 {
        self.dlq_retry_delay_ms.unwrap_or(500)
    }

    /// Get the batch size for receiving messages in DLQ operations
    pub fn batch_size(&self) -> u32 {
        self.dlq_batch_size.unwrap_or(10)
    }
}

impl UIConfig {
    /// Get the duration between animation frames for loading indicators
    pub fn loading_frame_duration_ms(&self) -> u64 {
        self.ui_loading_frame_duration_ms.unwrap_or(100)
    }
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
        self.key_alt_next_page.unwrap_or(']')
    }
    pub fn alt_prev_page(&self) -> char {
        self.key_alt_prev_page.unwrap_or('[')
    }

    // Message actions
    pub fn send_to_dlq(&self) -> char {
        self.key_send_to_dlq.unwrap_or('s')
    }
    pub fn resend_from_dlq(&self) -> char {
        self.key_resend_from_dlq.unwrap_or('s')
    }
    pub fn resend_and_delete_from_dlq(&self) -> char {
        self.key_resend_and_delete_from_dlq.unwrap_or('S')
    }
    pub fn delete_message(&self) -> char {
        self.key_delete_message.unwrap_or('X')
    }
    pub fn alt_delete_message(&self) -> char {
        self.key_alt_delete_message.unwrap_or('X')
    }

    // Message details actions
    pub fn copy_message(&self) -> char {
        self.key_copy_message.unwrap_or('c')
    }
    pub fn yank_message(&self) -> char {
        self.key_yank_message.unwrap_or('y')
    }
    pub fn send_edited_message(&self) -> char {
        self.key_send_edited_message.unwrap_or('s') // 's' key
    }
    pub fn replace_edited_message(&self) -> char {
        self.key_replace_edited_message.unwrap_or('s')
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

    // Confirmation keys
    // Message composition keys
    pub fn toggle_dlq(&self) -> char {
        self.key_toggle_dlq.unwrap_or('d')
    }
    pub fn compose_multiple(&self) -> char {
        self.key_compose_multiple.unwrap_or('m')
    }
    pub fn compose_single(&self) -> char {
        self.key_compose_single.unwrap_or('n') // Note: This will be used with Ctrl modifier
    }

    pub fn confirm_yes(&self) -> char {
        self.key_confirm_yes.unwrap_or('y')
    }
    pub fn confirm_no(&self) -> char {
        self.key_confirm_no.unwrap_or('n')
    }
}

impl LoggingConfig {
    pub fn level(&self) -> &str {
        self.level.as_deref().unwrap_or("info")
    }

    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }
}
