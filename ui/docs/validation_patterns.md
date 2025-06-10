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
let name_validator = StringLengthValidator::new("Theme Name")
    .min_length(1)
    .max_length(50);
name_validator.validate(theme_name)?;
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

## Examples

### Theme Validation
```rust
// Good: Specific validator with clear errors
let validator = ThemeNameValidator;
validator.validate(theme_name)
    .map_err(|e| AppError::Config(e.user_message()))?;
```

### Message Validation
```rust
// Good: Multi-layer validation
validator_chain
    .add_validator(NonEmptyStringValidator::new("Message"))
    .add_validator(MessageSizeValidator::new(max_size))
    .validate(message_content)?;
```

### User Input
```rust
// Good: Validate before processing
let count = validate_number_input(input, 1, 100)?;
process_bulk_operation(count);
```

## Quick Reference

- **String validation**: Use `StringLengthValidator`, `NonEmptyStringValidator`
- **Number validation**: Use `NumericRangeValidator<T>`
- **Custom validation**: Implement `Validator<T>` trait
- **Error conversion**: Implement `From<ValidationError> for AppError`
- **Multiple rules**: Use `ValidationChain` or multiple validator calls
