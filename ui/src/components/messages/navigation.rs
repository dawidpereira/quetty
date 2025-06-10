use super::component::Messages;
use tuirealm::MockComponent;
use tuirealm::command::{Cmd, Direction};

impl Messages {
    /// Move selection down with bounds checking
    pub fn move_down(&mut self) {
        let current = self.current_index();
        let max_index = self.message_count().saturating_sub(1);

        if current < max_index {
            self.component_mut().perform(Cmd::Move(Direction::Down));
        }
    }

    /// Move selection up with bounds checking
    pub fn move_up(&mut self) {
        let current = self.current_index();
        if current > 0 {
            self.component_mut().perform(Cmd::Move(Direction::Up));
        }
    }

    /// Page down with bounds checking
    pub fn page_down(&mut self) {
        let current = self.current_index();
        let max_index = self.message_count().saturating_sub(1);

        if current < max_index {
            self.component_mut().perform(Cmd::Scroll(Direction::Down));
            // Ensure we don't go beyond the last item
            let new_index = self.current_index();
            if new_index > max_index {
                // Reset to the last valid position
                let moves_back = new_index - max_index;
                for _ in 0..moves_back {
                    self.component_mut().perform(Cmd::Move(Direction::Up));
                }
            }
        }
    }

    /// Page up with bounds checking
    pub fn page_up(&mut self) {
        self.component_mut().perform(Cmd::Scroll(Direction::Up));
    }
}
