# User Guide

This comprehensive guide covers all aspects of using Quetty, from basic navigation to advanced features like bulk operations and message editing.

## Getting Started

### First Launch

When you start Quetty for the first time:

1. **Authentication**: You'll be prompted to authenticate with Azure
2. **Resource Selection**: Choose your subscription, resource group, and namespace
3. **Queue Selection**: Select the queue you want to manage
4. **Main Interface**: You'll see the message list interface

### Interface Overview

```
┌─ Quetty ─────────────────────────────────────────────────────────────┐
│ Namespace: my-servicebus | Queue: orders (10 messages)               │
├─────────────────────────────────────────────────────────────────────┤
│ # | ID              | Timestamp           | Size | Count | State     │
│ 1 | msg-001-abc     | 2024-01-15 10:30:22 | 2KB  |   1   | Active   │
│ 2 | msg-002-def     | 2024-01-15 10:31:45 | 1KB  |   1   | Active   │
│ 3 | msg-003-ghi     | 2024-01-15 10:32:10 | 3KB  |   2   | Active   │
├─────────────────────────────────────────────────────────────────────┤
│ Message Content Preview:                                             │
│ {                                                                   │
│   "orderId": "12345",                                              │
│   "customerId": "cust-789",                                        │
│   "items": [...]                                                   │
│ }                                                                   │
└─────────────────────────────────────────────────────────────────────┘
 q:quit h:help d:dlq n/p:page ↑↓:navigate Enter:details
```

### Interface Components

1. **Header**: Shows current namespace, queue, and message count
2. **Message List**: Tabular view of messages with key properties
3. **Preview Pane**: Shows content of the selected message
4. **Help Bar**: Quick reference for keyboard shortcuts

## Navigation

### Basic Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up in lists |
| `↓` / `j` | Move down in lists |
| `Enter` / `o` | Select item or open details |
| `Esc` | Go back or cancel |
| `PgUp` / `PgDn` | Scroll through long content |

### Global Actions

| Key | Action |
|-----|--------|
| `q` | Quit application |
| `h` | Show help screen |
| `t` | Open theme picker |
| `Ctrl+C` | Open configuration screen |

## Message Management

### Viewing Messages

#### Message List
- Messages are displayed in a table with sequence number, ID, timestamp, size, delivery count, and state
- Use `↑`/`↓` or `j`/`k` to navigate between messages
- Selected message content is automatically previewed in the bottom pane

#### Message Details
- Press `Enter` on any message to open detailed view
- Detailed view shows:
  - Complete message content with syntax highlighting
  - Message properties and metadata
  - System properties (message ID, sequence number, etc.)
  - Custom properties

#### Message Navigation in Details
| Key | Action |
|-----|--------|
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `←` / `→` | Move cursor left/right when editing |
| `PgUp` / `PgDn` | Page up/down |
| `Home` / `End` | Go to start/end |

### Message Operations

#### Deleting Messages
1. **Select Message**: Navigate to the message you want to delete
2. **Trigger Delete**: Press `Delete` or `Ctrl+X`
3. **Confirm**: Press `y` to confirm, `n` to cancel
4. **Result**: Message is permanently removed from the queue

> ⚠️ **Warning**: Message deletion is permanent and cannot be undone.

#### Copying Messages
In message details view:
- `y` - Yank (copy) message content to clipboard
- `Ctrl+C` - Copy message content to clipboard

### Pagination

Quetty uses smart client-side pagination for efficient browsing:

#### Page Navigation
| Key | Action |
|-----|--------|
| `n` / `]` | Next page |
| `p` / `[` | Previous page |
| `z` | Set page size (10-1000 messages) |

#### How Pagination Works
- **Smart Loading**: Only loads new messages when needed
- **Instant Backward Navigation**: Previous pages are cached
- **Memory Efficient**: Only keeps viewed messages in memory
- **Configurable**: Set page size via `z` key or configuration

## Dead Letter Queue (DLQ) Support

### DLQ Navigation
- Press `d` to toggle between main queue and dead letter queue
- DLQ name format: `your-queue-name/$deadletterqueue`
- All navigation and viewing features work the same in DLQ

### DLQ Operations

#### Sending Messages to DLQ
1. **Select Message**: Choose message in main queue
2. **Send to DLQ**: Press `Ctrl+D`
3. **Confirm**: Confirm the operation
4. **Result**: Message moves to dead letter queue

#### Resending from DLQ
1. **Navigate to DLQ**: Press `d` to switch to DLQ view
2. **Select Message**: Choose message to resend
3. **Resend**: Press `r` to resend to main queue
4. **Confirm**: Confirm the operation
5. **Result**: Message moves back to main queue

> 📝 **Note**: DLQ operations may take a few moments to complete due to Azure Service Bus processing.

## Bulk Operations

### Selecting Messages

#### Single Selection
- Use `Space` to toggle selection of current message
- Selected messages are highlighted

#### Multi-Selection
- `a` - Select all messages on current page
- `Space` - Toggle individual message selection
- Navigate with arrow keys while maintaining selection

#### Selection Indicators
- ✓ Selected messages show a checkmark
- Selection count displayed in status bar

### Bulk Delete
1. **Select Messages**: Use `Space` and `a` to select messages
2. **Trigger Bulk Delete**: Press `Ctrl+X` with multiple messages selected
3. **Review Selection**: Confirm the number of messages to delete
4. **Execute**: Confirm to permanently delete all selected messages

### Bulk DLQ Operations
1. **Select Messages**: Choose multiple messages
2. **Bulk Send to DLQ**: Press `Ctrl+D` with selection
3. **Confirm**: Review and confirm the operation
4. **Progress**: Watch real-time progress indicator

### Bulk Limits
- Maximum batch size: 200 messages (configurable)
- Maximum total messages: 10,000 per operation
- Operations have configurable timeouts for safety

## Message Editing and Composition

### Editing Existing Messages
1. **Open Message Details**: Press `Enter` on a message
2. **Enter Edit Mode**: Press `e` to start editing
3. **Edit Content**: Modify the message content
4. **Save Options**:
   - `Ctrl+S` - Send as new message (preserves original)
   - `Ctrl+R` - Replace original message (deletes original)
5. **Cancel**: Press `Esc` to cancel editing

### Composing New Messages
1. **Single Message**: Press `Ctrl+N`
2. **Multiple Messages**: Press `m` and specify count
3. **Enter Content**: Type or paste message content
4. **Send**: Press `Ctrl+S` to send
5. **Cancel**: Press `Esc` to cancel

### Message Validation
- JSON messages are automatically formatted and validated
- Syntax errors are highlighted
- Invalid messages cannot be sent until fixed

## Queue and Namespace Management

### Switching Queues
1. **Open Queue Picker**: Press `o` in main view
2. **Select Queue**: Choose from available queues
3. **Confirm**: Press `Enter` to switch

### Switching Namespaces
1. **Open Namespace Picker**: Press `Ctrl+O`
2. **Select Namespace**: Choose from available namespaces
3. **Authenticate**: Re-authenticate if needed
4. **Queue Selection**: Choose queue in new namespace

### Azure Resource Discovery
- Quetty automatically discovers available:
  - Subscriptions (if multiple)
  - Resource groups
  - Service Bus namespaces
  - Queues within namespaces

## Queue Statistics

### Statistics Display
- **Message Count**: Total messages in queue
- **Dead Letter Count**: Messages in DLQ
- **Active Messages**: Messages available for delivery
- **Scheduled Messages**: Messages scheduled for future delivery

### Real-time Updates
- Statistics refresh automatically
- Configurable refresh interval
- Can be disabled for performance

### Statistics Cache
- Results cached to reduce API calls
- Configurable cache duration
- Manual refresh available

## Themes and Customization

### Built-in Themes

#### Nightfox Family
- `nightfox` - Dark blue theme
- `duskfox` - Darker variant with purple accents
- `dawnfox` - Light theme
- `nordfox` - Nord-inspired colors
- `terafox` - Green accent theme
- `carbonfox` - Carbon-inspired dark theme

#### Catppuccin Family
- `mocha` - Dark theme
- `macchiato` - Medium dark
- `frappe` - Medium light
- `latte` - Light theme

#### Quetty Family
- `dark` - Custom dark theme
- `light` - Custom light theme

### Changing Themes
1. **Open Theme Picker**: Press `t`
2. **Browse Themes**: Navigate through available themes
3. **Preview**: See live preview of each theme
4. **Select**: Press `Enter` to apply theme
5. **Save**: Theme preference is saved automatically

### Theme Configuration
```toml
[theme]
theme_name = "nightfox"
flavor_name = "duskfox"
```

## Help System

### Context-Sensitive Help
- Press `h` at any time to see available shortcuts
- Help content changes based on current screen
- Shows both global and context-specific actions

### Help Categories
1. **Global Actions**: Quit, help, theme
2. **Navigation**: Movement, selection, pagination
3. **Message Operations**: Delete, copy, edit
4. **DLQ Operations**: Send, resend, toggle
5. **Bulk Operations**: Selection, bulk actions

## Performance Tips

### Large Queues
- Use larger page sizes (100-500) for faster browsing
- Enable statistics caching
- Consider filtering (future feature)

### Network Optimization
- Adjust poll timeouts based on connection speed
- Use appropriate DLQ timeouts
- Consider operation timeouts for bulk operations

### Memory Management
- Quetty only keeps viewed messages in memory
- Clear cache by restarting application
- Use smaller page sizes if memory is limited

## Keyboard Shortcuts Reference

### Global Shortcuts
| Key | Action |
|-----|--------|
| `q` | Quit application |
| `h` | Show help |
| `t` | Theme picker |
| `Ctrl+C` | Configuration screen |
| `Esc` | Cancel/Go back |

### Navigation
| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `←` / `→` | Move left/right (in text) |
| `Enter` / `o` | Select/Open |
| `PgUp` / `PgDn` | Page up/down |

### Message Operations
| Key | Action |
|-----|--------|
| `Delete` / `Ctrl+X` | Delete message(s) |
| `y` | Copy/yank message |
| `Ctrl+C` | Copy message |
| `e` | Edit message |
| `Ctrl+S` | Send edited message |
| `Ctrl+R` | Replace message |

### Dead Letter Queue
| Key | Action |
|-----|--------|
| `d` | Toggle DLQ view |
| `Ctrl+D` | Send to DLQ |
| `r` | Resend from DLQ |

### Pagination
| Key | Action |
|-----|--------|
| `n` / `]` | Next page |
| `p` / `[` | Previous page |
| `z` | Set page size |

### Bulk Operations
| Key | Action |
|-----|--------|
| `Space` | Toggle selection |
| `a` | Select all on page |
| `Ctrl+X` | Bulk delete |
| `Ctrl+D` | Bulk send to DLQ |

### Message Composition
| Key | Action |
|-----|--------|
| `Ctrl+N` | New message |
| `m` | Multiple messages |
| `Ctrl+S` | Send message |

### Queue Management
| Key | Action |
|-----|--------|
| `o` | Queue picker |
| `Ctrl+O` | Namespace picker |

## Troubleshooting

### Common Issues

#### Authentication Problems
- Check Azure AD configuration
- Verify connection string format
- Ensure proper permissions

#### Performance Issues
- Reduce page size for large queues
- Adjust polling intervals
- Check network connectivity

#### UI Issues
- Try different themes
- Check terminal compatibility
- Verify Unicode support

### Getting Help
1. **Built-in Help**: Press `h` for context-specific help
2. **Logs**: Enable debug logging for detailed information
3. **Documentation**: See [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
4. **GitHub Issues**: Report bugs and feature requests

## Configuration

For detailed configuration options, see [CONFIGURATION.md](CONFIGURATION.md).

### Quick Configuration
```toml
# Basic performance tuning
page_size = 100
poll_timeout_ms = 10

# Theme selection
[theme]
theme_name = "nightfox"
flavor_name = "duskfox"

# Authentication
[azure_ad]
auth_method = "device_code"
tenant_id = "your-tenant-id"
client_id = "your-client-id"
```

## Advanced Features

### Custom Key Bindings
All keyboard shortcuts can be customized in the configuration file:

```toml
[keys]
key_quit = "q"
key_help = "?"
key_delete_message = "x"
# ... more customizations
```

### Environment-Specific Configuration
Use environment variables for different environments:

```bash
# Development
export QUETTY_PAGE_SIZE=10
export LOGGING__LEVEL="debug"

# Production
export QUETTY_PAGE_SIZE=200
export LOGGING__LEVEL="warn"
```

### Logging and Debugging
Enable detailed logging for troubleshooting:

```toml
[logging]
level = "debug"
file = "quetty.log"
```

## Best Practices

### Security
- Use Azure AD authentication when possible
- Never commit connection strings to version control
- Regularly rotate credentials

### Performance
- Use appropriate page sizes for your queue volume
- Enable statistics caching for frequently accessed queues
- Monitor Azure Service Bus throttling limits

### Workflow
- Use bulk operations for managing multiple messages
- Leverage DLQ for message troubleshooting
- Take advantage of message editing for testing
- Use themes and customization for comfortable long-term use

For additional help and advanced configuration, see the other documentation files:
- [INSTALLATION.md](INSTALLATION.md) - Setup and installation
- [AUTHENTICATION.md](AUTHENTICATION.md) - Authentication configuration
- [CONFIGURATION.md](CONFIGURATION.md) - Complete configuration reference
- [THEMING.md](THEMING.md) - Theme development and customization
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Problem resolution
