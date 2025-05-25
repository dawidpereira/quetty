# Quetty

## Overview
Quetty is a terminal-based queue manager designed to help you manage and interact with message queues efficiently. It provides a user-friendly interface for viewing, previewing, and managing messages in your queues.

## Features
- **Message Preview**: Automatically previews the first message when messages are loaded.
- **Interactive UI**: Navigate through messages using keyboard shortcuts.
- **Queue Management**: Select and manage different queues and namespaces.
- **Real-time Updates**: Load and display messages in real-time.
- **Message Pagination**: Efficiently browse through large message queues with client-side pagination.
- **Smart Caching**: Previously viewed pages are cached for instant navigation.

## Installation
1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/quetty.git
   cd quetty
   ```
2. Build the application:
   ```bash
   cd ui
   cargo build
   ```

## Project Structure
- `ui/` - Terminal user interface (main application)
- `server/` - Core library for Azure Service Bus integration
- `ui/config.toml` - Configuration file for the application

## Usage
Run the application:
```bash
cd quetty/ui
cargo run
```

### Keyboard Shortcuts
- **Navigation**
  - **↑/k**: Move up in lists or text
  - **↓/j**: Move down in lists or text
  - **←/→**: Move cursor left/right in message details
  - **Enter/o**: Select an item
  - **Esc**: Go back/cancel the current action
  - **PgUp/PgDn**: Scroll through long content
- **Message Pagination**
  - **n/]**: Go to next page of messages
  - **p/[**: Go to previous page of messages
- **Global**
  - **q**: Quit the application
  - **h**: Show help screen with keyboard shortcuts

## Message Pagination

Quetty implements an efficient client-side pagination system for browsing through large message queues:

### How it works:
- **Page Size**: Configurable via `max_messages` in `config.toml` (default: 10 messages per page)
- **Smart Loading**: Only loads new messages from the API when needed
- **Local Caching**: Previously viewed pages are stored locally for instant navigation
- **Memory Efficient**: Only keeps messages you've actually viewed

### Navigation:
- **Next Page (`n` or `]`)**: 
  - If the page is already loaded → instant switch
  - If new page needed → loads from API and advances
- **Previous Page (`p` or `[`)**: 
  - Always instant using cached messages
  - No API calls required

### Benefits:
- **Fast backward navigation** - no API delays
- **Consistent view** - shows messages as they were when loaded
- **Handles queue changes** - new messages only affect future loads
- **Reliable pagination** - no complex sequence tracking

## Configuration

Edit `ui/config.toml` to customize pagination:
```toml
# Number of messages per page
max_messages = 10
```

## Next Steps

### Error Handling & Logging:
- [x] Implement robust error handling for API calls, file operations, and user interactions.
- [x] Add logging to help with debugging and monitoring the application's behavior.
### User Experience Enhancements:
- [x] Improve the UI/UX with better feedback (e.g., loading indicators, success/error messages).
- [x] Add keyboard shortcuts or tooltips to make navigation more intuitive.
### Testing:
- [ ] Write unit tests for critical components and integration tests for key workflows.
- [ ] Ensure edge cases (e.g., empty message lists, network failures) are handled gracefully.
### Documentation:
- [ ] Document the codebase, especially complex logic or public APIs.
- [x] Create a user guide or README to help new users get started.
### Feature Expansion:
- [x] Support for messages pagination.
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
