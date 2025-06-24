use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueType {
    Main,
    DeadLetter,
}

impl QueueType {
    pub fn from_queue_name(queue_name: &str) -> Self {
        if queue_name.ends_with("/$deadletterqueue") {
            QueueType::DeadLetter
        } else {
            QueueType::Main
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueInfo {
    pub name: String,
    pub queue_type: QueueType,
}

impl QueueInfo {
    pub fn new(name: String, queue_type: QueueType) -> Self {
        Self { name, queue_type }
    }

    pub fn main_queue(name: String) -> Self {
        Self::new(name, QueueType::Main)
    }

    pub fn dead_letter_queue(base_name: String) -> Self {
        let dlq_name = format!("{}/$deadletterqueue", base_name);
        Self::new(dlq_name, QueueType::DeadLetter)
    }

    pub fn base_name(&self) -> String {
        match self.queue_type {
            QueueType::Main => self.name.clone(),
            QueueType::DeadLetter => {
                if self.name.ends_with("/$deadletterqueue") {
                    self.name
                        .strip_suffix("/$deadletterqueue")
                        .unwrap()
                        .to_string()
                } else {
                    self.name.clone()
                }
            }
        }
    }

    pub fn to_dlq(&self) -> Self {
        Self::dead_letter_queue(self.base_name())
    }

    pub fn to_main(&self) -> Self {
        Self::main_queue(self.base_name())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageData {
    pub content: String,
    pub properties: Option<std::collections::HashMap<String, String>>,
}

impl MessageData {
    pub fn new(content: String) -> Self {
        Self {
            content,
            properties: None,
        }
    }

    pub fn with_properties(
        content: String,
        properties: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            content,
            properties: Some(properties),
        }
    }
}

/// Statistics about service bus operations
#[derive(Debug, Clone, Default)]
pub struct OperationStats {
    pub successful: usize,
    pub failed: usize,
    pub total: usize,
}

impl OperationStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_success(&mut self) {
        self.successful += 1;
        self.total += 1;
    }

    pub fn add_failure(&mut self) {
        self.failed += 1;
        self.total += 1;
    }

    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.successful as f64 / self.total as f64
        }
    }
}
