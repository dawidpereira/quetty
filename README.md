# Quetty

⚠️ **DEVELOPMENT STATUS**: This application is currently under active development and is **NOT ready for production use**. Features may be incomplete, unstable, or subject to breaking changes. Use at your own risk and thoroughly test in development environments only.

## Overview
Quetty is a terminal-based queue manager designed to help you manage and interact with message queues efficiently. It provides a user-friendly interface for viewing, previewing, and managing messages in your queues.

## Features
- **Message Preview**: Automatically previews the first message when messages are loaded.
- **Interactive UI**: Navigate through messages using keyboard shortcuts.
- **Queue Management**: Select and manage different queues and namespaces.
- **Real-time Updates**: Load and display messages in real-time.
- **Message Pagination**: Efficiently browse through large message queues with client-side pagination.
- **Smart Caching**: Previously viewed pages are cached for instant navigation.
- **Dead Letter Queue (DLQ) Support**: Switch between main queue and DLQ, send messages to DLQ.
- **Message Operations**: Delete messages from queue with confirmation dialogs.
- **Smart State Management**: Local state updates for instant UI refresh without server calls.

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
  - **z**: Select page size (100-1000 messages per page)
- **Message Operations**
  - **Delete/Ctrl+X**: Delete selected message from queue (with confirmation)
- **Dead Letter Queue**
  - **d**: Toggle between main queue and dead letter queue
  - **Ctrl+D**: Send selected message to dead letter queue (with confirmation)
  - **r**: Resend message from dead letter queue to main queue (DLQ only, development)
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

## Dead Letter Queue (DLQ) Support

⚠️ **Development Feature Warning**: DLQ **message sending** functionality is currently in development and is **NOT recommended for production use**. Use message sending with caution and thoroughly test in development environments only.

Quetty provides comprehensive support for Azure Service Bus Dead Letter Queues:

### Features:
- **Queue Switching**: Toggle between main queue and its dead letter queue using the `d` key
- **Message Transfer**: Send messages to DLQ using `Ctrl+D` with confirmation dialog
- **Message Resending**: Resend messages from DLQ back to main queue using `r` key (development)
- **Producer Integration**: Uses dedicated Producer module for clean message sending
- **Smart State Management**: Instant UI updates without server reloads
- **Precise Targeting**: Messages are matched by both ID and sequence number for accuracy

### How it works:
1. **View DLQ**: Press `d` to switch between main queue and dead letter queue
2. **Send to DLQ**: Select a message and press `Ctrl+D` to send it to the dead letter queue
3. **Resend from DLQ**: Select a message in DLQ and press `r` to resend it to the main queue
4. **Confirmation**: A popup asks for confirmation before any operation
5. **Instant Update**: The message is removed from the current view immediately
6. **Local State**: No server reload needed - the UI updates instantly

### Queue Naming:
- **Main Queue**: `your-queue-name`
- **Dead Letter Queue**: `your-queue-name/$deadletterqueue`

### Safety Features:
- **Confirmation Dialog**: Prevents accidental DLQ operations
- **Dual Matching**: Messages are identified by both ID and sequence number
- **Error Handling**: Comprehensive error reporting for failed operations
- **State Consistency**: Local state always matches server state

### Development Status:
- ✅ DLQ switching and viewing
- ✅ Queue navigation
- ✅ Smart local state management
- ⚠️ **Message transfer to DLQ** - Under development
- ⚠️ **Message resending from DLQ** - Under development
- ⚠️ **DLQ operations error recovery** - Limited testing
- ⚠️ **DLQ operations edge cases** - May not handle all scenarios

## Message Operations

### Delete Messages

Quetty allows you to permanently delete messages from both main queues and dead letter queues:

#### Features:
- **Safe Deletion**: Confirmation dialog prevents accidental deletions
- **Dual Shortcuts**: Use either `Delete` key or `Ctrl+X` for deletion
- **Queue Support**: Works on both main queue and dead letter queue messages
- **Instant Feedback**: Loading indicators and immediate UI updates
- **Error Handling**: Comprehensive error reporting for failed operations

#### How it works:
1. **Select Message**: Navigate to the message you want to delete
2. **Trigger Delete**: Press `Delete` key or `Ctrl+X`
3. **Confirm Action**: A popup asks "Are you sure you want to delete this message from the queue?"
4. **Loading Indicator**: Shows "Deleting message from queue..." during operation
5. **State Update**: Message is immediately removed from the UI upon success

#### Safety Features:
- **Confirmation Required**: Cannot delete without explicit confirmation
- **Clear Warning**: Popup clearly states the action is permanent and cannot be undone
- **Precise Targeting**: Messages are identified by both ID and sequence number
- **Error Recovery**: Failed deletions are reported with detailed error messages

#### Technical Details:
- **Operation**: Uses Azure Service Bus `complete_message` to permanently remove messages
- **State Management**: Local message list is updated immediately upon successful deletion
- **Shared Utilities**: Uses the same message finding logic as DLQ operations for consistency

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
- [x] Support for DLQ (sending: development stage).
##### Message management
- [x] DQL message
- [x] Resend message from DLQ (development)
- [x] Delete message
- [ ] Bulk DLQ
- [ ] Bulk delete
- [ ] Bulk resend
- [ ] Edit message
- [ ] Send new message

## License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
