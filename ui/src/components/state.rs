use crate::error::{AppError, AppResult};

/// Trait for managing component lifecycle and state
pub trait ComponentState {
    /// Initialize component and prepare it for use
    fn mount(&mut self) -> AppResult<()>;
}

/// Extension trait for our specific Application type to mount components with ComponentState automatically
pub trait ComponentStateMount {
    /// Mount a component that implements ComponentState, calling mount() automatically
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

    /// Remount a component that implements ComponentState, calling mount() automatically
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
