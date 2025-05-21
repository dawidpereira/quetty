# Quetty

## Overview
Quetty is a terminal-based queue manager designed to help you manage and interact with message queues efficiently. It provides a user-friendly interface for viewing, previewing, and managing messages in your queues.

## Features
- **Message Preview**: Automatically previews the first message when messages are loaded.
- **Interactive UI**: Navigate through messages using keyboard shortcuts.
- **Queue Management**: Select and manage different queues and namespaces.
- **Real-time Updates**: Load and display messages in real-time.

## Installation
1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/quetty.git
   cd quetty
   ```
2. Build the application:
   ```bash
   cargo build
   ```

## Usage
Run the application:
```bash
cd quetty/ui
cargo run
```

### Keyboard Shortcuts
- **Up/Down/J/L**: Navigate through messages.
- **Enter**: Select a message for detailed view.
- **Esc**: Cancel the current action.

## Next Steps

### Error Handling & Logging:
- [ ] Implement robust error handling for API calls, file operations, and user interactions.
- [ ] Add logging to help with debugging and monitoring the application's behavior.
### User Experience Enhancements:
- [ ] Improve the UI/UX with better feedback (e.g., loading indicators, success/error messages).
- [ ] Add keyboard shortcuts or tooltips to make navigation more intuitive.
### Testing:
- [ ] Write unit tests for critical components and integration tests for key workflows.
- [ ] Ensure edge cases (e.g., empty message lists, network failures) are handled gracefully.
### Documentation:
- [ ] Document the codebase, especially complex logic or public APIs.
- [ ] Create a user guide or README to help new users get started.
### Feature Expansion:
- [ ] Support for messages pagination.
- [ ] Support for DLQ.
##### Message management
- [ ] Delete message
- [ ] Bulk delete
- [ ] Resend message
- [ ] Bulk resend
- [ ] Edit message
- [ ] Send new message

## License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 
