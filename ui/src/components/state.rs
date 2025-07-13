//! Component state management and lifecycle traits.
//!
//! This module provides traits for managing component lifecycle and state in the
//! Quetty terminal user interface. It establishes patterns for component initialization
//! and integration with the TUI realm framework.

use crate::error::{AppError, AppResult};

/// Trait for managing component lifecycle and state initialization.
///
/// Provides a standardized way to handle component initialization and setup.
/// Components implementing this trait can perform any necessary initialization
/// work before being displayed to the user.
///
/// # Examples
///
/// ```no_run
/// use quetty::components::state::ComponentState;
/// use quetty::error::AppResult;
///
/// struct MyComponent {
///     data: Vec<String>,
/// }
///
/// impl ComponentState for MyComponent {
///     fn mount(&mut self) -> AppResult<()> {
///         // Load initial data
///         self.data = vec!["Item 1".to_string(), "Item 2".to_string()];
///         Ok(())
///     }
/// }
/// ```
pub trait ComponentState {
    /// Initializes the component and prepares it for use.
    ///
    /// This method is called before the component is displayed to perform
    /// any necessary setup work such as loading data, initializing state,
    /// or configuring the component.
    ///
    /// # Returns
    ///
    /// `Ok(())` if initialization succeeds, or an [`AppError`] if it fails
    ///
    /// # Errors
    ///
    /// Returns an error if component initialization fails for any reason
    fn mount(&mut self) -> AppResult<()>;
}

/// Extension trait for mounting components with automatic state initialization.
///
/// Provides convenient methods for mounting components that implement [`ComponentState`]
/// to the TUI realm application. Automatically calls the component's [`mount()`] method
/// during the mounting process to ensure proper initialization.
///
/// # Examples
///
/// ```ignore
/// use quetty::components::state::{ComponentState, ComponentStateMount};
/// use quetty::components::common::ComponentId;
///
/// // Mount a component with automatic state initialization
/// app.mount_with_state(
///     ComponentId::Messages,
///     my_component,
///     vec![]
/// )?;
/// ```
pub trait ComponentStateMount {
    /// Mounts a component with automatic state initialization.
    ///
    /// This method combines component state initialization with TUI realm mounting.
    /// It first calls [`ComponentState::mount()`] on the component, then mounts
    /// it to the TUI realm with the specified subscriptions.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the component
    /// * `component` - The component to mount (must implement [`ComponentState`])
    /// * `subs` - Event subscriptions for the component
    ///
    /// # Returns
    ///
    /// `Ok(())` if mounting succeeds, or an [`AppError`] if it fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Component state initialization fails
    /// - TUI realm mounting fails
    fn mount_with_state<C>(
        &mut self,
        id: crate::components::common::ComponentId,
        component: C,
        subs: Vec<tuirealm::Sub<crate::components::common::ComponentId, tuirealm::NoUserEvent>>,
    ) -> AppResult<()>
    where
        C: ComponentState
            + tuirealm::MockComponent
            + tuirealm::Component<crate::components::common::Msg, tuirealm::NoUserEvent>
            + 'static;

    /// Remounts a component with automatic state initialization.
    ///
    /// Similar to [`mount_with_state`], but replaces an existing component with
    /// the same ID. Useful for refreshing components or switching between different
    /// implementations of the same component type.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the component (must already exist)
    /// * `component` - The new component to mount (must implement [`ComponentState`])
    /// * `subs` - Event subscriptions for the component
    ///
    /// # Returns
    ///
    /// `Ok(())` if remounting succeeds, or an [`AppError`] if it fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Component state initialization fails
    /// - TUI realm remounting fails
    /// - No component exists with the specified ID
    fn remount_with_state<C>(
        &mut self,
        id: crate::components::common::ComponentId,
        component: C,
        subs: Vec<tuirealm::Sub<crate::components::common::ComponentId, tuirealm::NoUserEvent>>,
    ) -> AppResult<()>
    where
        C: ComponentState
            + tuirealm::MockComponent
            + tuirealm::Component<crate::components::common::Msg, tuirealm::NoUserEvent>
            + 'static;
}

impl ComponentStateMount
    for tuirealm::Application<
        crate::components::common::ComponentId,
        crate::components::common::Msg,
        tuirealm::NoUserEvent,
    >
{
    fn mount_with_state<C>(
        &mut self,
        id: crate::components::common::ComponentId,
        mut component: C,
        subs: Vec<tuirealm::Sub<crate::components::common::ComponentId, tuirealm::NoUserEvent>>,
    ) -> AppResult<()>
    where
        C: ComponentState
            + tuirealm::MockComponent
            + tuirealm::Component<crate::components::common::Msg, tuirealm::NoUserEvent>
            + 'static,
    {
        // Initialize component using ComponentState pattern
        component.mount()?;

        // Mount to TUI realm
        self.mount(id, Box::new(component), subs)
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }

    fn remount_with_state<C>(
        &mut self,
        id: crate::components::common::ComponentId,
        mut component: C,
        subs: Vec<tuirealm::Sub<crate::components::common::ComponentId, tuirealm::NoUserEvent>>,
    ) -> AppResult<()>
    where
        C: ComponentState
            + tuirealm::MockComponent
            + tuirealm::Component<crate::components::common::Msg, tuirealm::NoUserEvent>
            + 'static,
    {
        // Initialize component using ComponentState pattern
        component.mount()?;

        // Remount to TUI realm
        self.remount(id, Box::new(component), subs)
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }
}
