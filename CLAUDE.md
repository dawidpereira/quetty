# Claude Development Guidelines for Quetty

This document contains important guidelines for Claude or other AI assistants when working on the Quetty codebase.

## Logging Guidelines

### No Emojis in Logs
- **DO NOT** use emojis in log messages (e.g., `log::info!`, `log::debug!`, `log::error!`)
- Emojis are acceptable in UI components and user-facing popups
- Keep log messages professional and text-only

### No Sensitive Data in Logs
- **NEVER** log sensitive information such as:
  - Authentication tokens
  - User codes
  - Client secrets
  - Connection strings
  - Personal identifiable information (PII)
  - URLs containing sensitive parameters
  
### User-Facing Output
- Use the UI layer (popups, messages) for information that needs to be shown to users
- Do not use `println!` or similar console output for user communication
- Route user notifications through the proper UI components

## Example of What NOT to Do

```rust
// BAD - Don't do this:
log::info!("🔐 Authentication successful!");
log::info!("User code: {}", device_code.user_code);
println!("Please visit: {}", auth_url);

// GOOD - Do this instead:
log::info!("Authentication successful");
log::info!("Device code authentication initiated - awaiting user action");
// Show sensitive info through UI components, not logs
```

## Authentication and Security

- Authentication credentials should be stored in `.env` files
- Configuration files (`config.toml`) should only contain non-sensitive settings
- Use the UI layer to display authentication prompts or codes to users
- Log authentication events without revealing sensitive details

## Code Quality

- Run lint and typecheck commands before completing tasks
- Follow existing code patterns and conventions
- Use the existing error handling mechanisms
- Maintain consistency with the codebase style

## Code Comments

- **DO NOT** include comments that indicate code was refactored, changed, updated, or simplified
- Keep comments focused on explaining the "what" and "why" of the current implementation
- Avoid temporal references like "now", "previously", "after refactoring", etc.
- Comments should describe the current state of the code, not its history

## Backward Compatibility

- **DO NOT** maintain backward compatibility unless explicitly requested
- Remove deprecated parameters and functions immediately
- Clean up old API signatures without preserving them
- If backward compatibility is needed, it will be explicitly specified in the request