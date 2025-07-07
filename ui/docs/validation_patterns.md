# Validation Guidelines

## Quick Start

Use the `Validator` trait for all validation needs:

```rust
use crate::validation::Validator;

impl Validator<str> for MyValidator {
    type Error = MyError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        // Your validation logic
        Ok(())
    }
}
```

## When to Validate

1. **User Input** - Before processing any user-provided data
2. **External Data** - When loading files, configs, or API responses
3. **Critical Operations** - Before bulk operations or destructive actions
4. **State Changes** - When updating application state

## Validation Layers

### 1. Input Validation
```rust
// Validate basic input constraints
let content_validator = MessageContentValidator;
content_validator.validate(message_content)?;

let size_validator = MessageSizeValidator::azure_default();
size_validator.validate(message_content)?;
```

### 2. Business Logic Validation
```rust
// Validate business rules
if !theme_exists(theme_name) {
    return Err(ThemeError::NotFound);
}
```

### 3. System Validation
```rust
// Validate system constraints
if !has_permission(user, operation) {
    return Err(SecurityError::Unauthorized);
}
```

## Error Handling

Create structured errors with user-friendly messages:

```rust
#[derive(Debug, Clone)]
pub enum MyValidationError {
    InvalidInput { field: String, reason: String },
    MissingRequired { field: String },
}

impl MyValidationError {
    pub fn user_message(&self) -> String {
        match self {
            Self::InvalidInput { field, reason } =>
                format!("Invalid {}: {}", field, reason),
            Self::MissingRequired { field } =>
                format!("{} is required", field),
        }
    }
}
```

## Best Practices

### ✅ Do
- Use specific error types with context
- Validate early and fail fast
- Provide clear error messages to users
- Test validation logic thoroughly
- Combine validators for complex rules

### ❌ Don't
- Use generic `String` errors
- Validate after side effects
- Expose technical details to users
- Skip validation for "trusted" inputs
- Duplicate validation logic

## Current Implementations

### Theme Validation
```rust
// Validate theme names and configurations
let validator = ThemeNameValidator;
validator.validate(theme_name)
    .map_err(|e| AppError::Config(e.user_message()))?;
```

### Message Content Validation
```rust
// Comprehensive message validation
let validator = CompleteMessageValidator::azure_default();
validator.validate(message_content)?;

// Individual validators
let content_validator = MessageContentValidator;      // Non-empty
let size_validator = MessageSizeValidator::new(limit); // Size limits
let json_validator = JsonFormatValidator;             // JSON format
let encoding_validator = MessageEncodingValidator;    // UTF-8 encoding
```

### Number Input Validation
```rust
// Numeric range validation with user-friendly errors
let validator = NumericRangeValidator::new(min, max);
validator.validate(user_input)?;
```

### Configuration Validation
```rust
// Validate app configuration against limits
config.validate()
    .map_err(|errors| show_config_errors(errors))?;
```

## Quick Reference

- **String validation**: Use `MessageContentValidator`, `MessageSizeValidator`
- **Number validation**: Use `NumericRangeValidator<T>`
- **JSON validation**: Use `JsonFormatValidator`
- **Theme validation**: Use `ThemeNameValidator`, `FlavorNameValidator`
- **Custom validation**: Implement `Validator<T>` trait
- **Error conversion**: Implement `From<ValidationError> for AppError`
- **Composite validation**: Use multiple validator calls or create composite validators
