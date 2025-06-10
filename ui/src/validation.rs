/// Core validation trait that all validators must implement.
///
/// This trait provides a consistent interface for validating data across
/// the application. Validators can be composed and chained together for
/// complex validation scenarios.
///
/// # Type Parameters
///
/// * `T` - The type of data being validated (can be unsized like `str`)
///
/// # Examples
///
/// ```
/// use quetty::validation::Validator;
///
/// struct MyValidator;
/// impl Validator<str> for MyValidator {
///     type Error = String;
///
///     fn validate(&self, input: &str) -> Result<(), Self::Error> {
///         if input.is_empty() {
///             Err("Input cannot be empty".to_string())
///         } else {
///             Ok(())
///         }
///     }
/// }
/// ```
pub trait Validator<T: ?Sized> {
    type Error;

    /// Validate the input and return Ok(()) if valid, or Err with validation error
    fn validate(&self, input: &T) -> Result<(), Self::Error>;
}
