use entity_table::ComponentTable;
pub use entity_table::{ComponentTableIter, ComponentTableIterMut, Entity};
#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A component of an entity which can produce realtime events
pub trait RealtimeComponent {
    /// Events that will be periodically emited by this component
    type Event;

    /// Generate an event, along with the time until the next tick should take place
    fn tick(&mut self) -> (Self::Event, Duration);
}

pub trait RealtimeComponentApplyEvent<C>: RealtimeComponent {
    /// Apply an event to a context. This is separated from `tick` so that the context
    /// can include the container of this `RealtimeComponent`.
    fn apply_event(event: <Self as RealtimeComponent>::Event, entity: Entity, context: &mut C);
}

#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct ScheduledRealtimeComponent<T: RealtimeComponent> {
    pub component: T,
    pub until_next_tick: Duration,
}

#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct RealtimeComponentTable<T: RealtimeComponent>(
    ComponentTable<ScheduledRealtimeComponent<T>>,
);

impl<T: RealtimeComponent> Default for RealtimeComponentTable<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: RealtimeComponent> RealtimeComponentTable<T> {
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn insert_with_schedule(
        &mut self,
        entity: Entity,
        data: ScheduledRealtimeComponent<T>,
    ) -> Option<ScheduledRealtimeComponent<T>> {
        self.0.insert(entity, data)
    }
    pub fn insert(&mut self, entity: Entity, data: T) -> Option<T> {
        self.insert_with_schedule(
            entity,
            ScheduledRealtimeComponent {
                component: data,
                until_next_tick: Duration::from_millis(0),
            },
        )
        .map(|c| c.component)
    }
    pub fn contains(&self, entity: Entity) -> bool {
        self.0.contains(entity)
    }
    pub fn remove_with_schedule(
        &mut self,
        entity: Entity,
    ) -> Option<ScheduledRealtimeComponent<T>> {
        self.0.remove(entity)
    }
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        self.remove_with_schedule(entity).map(|c| c.component)
    }
    pub fn get_with_schedule(&self, entity: Entity) -> Option<&ScheduledRealtimeComponent<T>> {
        self.0.get(entity)
    }
    pub fn get_with_schedule_mut(
        &mut self,
        entity: Entity,
    ) -> Option<&mut ScheduledRealtimeComponent<T>> {
        self.0.get_mut(entity)
    }
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.get_with_schedule(entity).map(|c| &c.component)
    }
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.get_with_schedule_mut(entity).map(|c| &mut c.component)
    }
    pub fn iter_with_schedule(&self) -> ComponentTableIter<ScheduledRealtimeComponent<T>> {
        self.0.iter()
    }
    pub fn iter_with_schedule_mut(
        &mut self,
    ) -> ComponentTableIterMut<ScheduledRealtimeComponent<T>> {
        self.0.iter_mut()
    }
    pub fn iter(&self) -> RealtimeComponentTableIter<T> {
        RealtimeComponentTableIter(self.0.iter())
    }
    pub fn iter_mut(&mut self) -> RealtimeComponentTableIterMut<T> {
        RealtimeComponentTableIterMut(self.0.iter_mut())
    }
    pub fn entities(&self) -> impl '_ + Iterator<Item = Entity> {
        self.iter().map(|(entity, _)| entity)
    }
}

pub struct RealtimeComponentTableIter<'a, T: RealtimeComponent>(
    ComponentTableIter<'a, ScheduledRealtimeComponent<T>>,
);

pub struct RealtimeComponentTableIterMut<'a, T: RealtimeComponent>(
    ComponentTableIterMut<'a, ScheduledRealtimeComponent<T>>,
);

impl<'a, T: RealtimeComponent> Iterator for RealtimeComponentTableIter<'a, T> {
    type Item = (Entity, &'a T);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(entity, c)| (entity, &c.component))
    }
}

impl<'a, T: RealtimeComponent> Iterator for RealtimeComponentTableIterMut<'a, T> {
    type Item = (Entity, &'a mut T);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(entity, c)| (entity, &mut c.component))
    }
}

pub trait ContextContainsRealtimeComponents {
    type Components: RealtimeComponents<Self>;
    fn components_mut(&mut self) -> &mut Self::Components;
}

pub trait RealtimeEntityEvents<C: ?Sized> {
    fn apply(self, entity: Entity, context: &mut C);
}

pub trait RealtimeComponents<C: ?Sized> {
    type EntityEvents: RealtimeEntityEvents<C>;

    fn tick_entity(
        &mut self,
        entity: Entity,
        frame_remaining: Duration,
    ) -> (Self::EntityEvents, Duration);
}

pub fn process_entity_frame<C: ContextContainsRealtimeComponents>(
    entity: Entity,
    frame_duration: Duration,
    context: &mut C,
) {
    let mut frame_remaining = frame_duration;
    while frame_remaining > Duration::from_micros(0) {
        let (events, until_next_tick) = context
            .components_mut()
            .tick_entity(entity, frame_remaining);
        events.apply(entity, context);
        frame_remaining -= until_next_tick;
    }
}

#[macro_export]
macro_rules! declare_realtime_entity_module {
    { $module_name:ident[$context:ty] { $($component_name:ident: $component_type:ty,)* } } => {
        $crate::declare_realtime_entity_module! { $module_name<>[$context] { $($component_name: $component_type,)* } }
    };
    { $module_name:ident<$lt:lifetime>[$context:ty] { $($component_name:ident: $component_type:ty,)* } } => {
        $crate::declare_realtime_entity_module! { $module_name<$lt,>[$context] { $($component_name: $component_type,)* } }
    };
    { $module_name:ident<$($lt:lifetime,)*>[$context:ty] { $($component_name:ident: $component_type:ty,)* } } => {
        mod $module_name {
            #[allow(unused_imports)]
            use super::*;

            /// Struct where each field contains a table associating entities with data
            /// (ie. components)
            #[derive(Debug, Clone)]
            pub struct RealtimeComponents {
                $(pub $component_name: $crate::RealtimeComponentTable<$component_type>,)*
            }

            impl Default for RealtimeComponents {
                fn default() -> Self {
                    Self {
                        $($component_name: Default::default(),)*
                    }
                }
            }

            /// Struct holding all components for a single entity
            #[derive(Debug, Clone)]
            pub struct RealtimeEntityData {
                $(pub $component_name: Option<$component_type>,)*
            }

            impl Default for RealtimeEntityData {
                fn default() -> Self {
                    Self {
                        $($component_name: None,)*
                    }
                }
            }

            /// Struct holding events associated with components for a given entity
            pub struct RealtimeEntityEvents {
                $(pub $component_name: Option<<$component_type as $crate::RealtimeComponent>::Event>,)*
            }

            impl RealtimeEntityEvents {
                /// Update a context by applying all the events.
                #[allow(unused)]
                pub fn apply<$($lt,)*>(
                    self,
                    entity: entity_table::Entity,
                    context: &mut $context,
                ) {
                    $(if let Some(event) = self.$component_name {
                        <$component_type as $crate::RealtimeComponentApplyEvent<$context>>::apply_event(
                            event,
                            entity,
                            context,
                        );
                    })*
                }
            }

            impl<$($lt,)*> $crate::RealtimeEntityEvents<$context> for RealtimeEntityEvents {
                fn apply(self, entity: Entity, context: &mut $context) {
                    RealtimeEntityEvents::apply(self, entity, context);
                }
            }

            impl RealtimeComponents {

                /// Remove all components for all entities.
                #[allow(unused)]
                pub fn clear(&mut self) {
                    $(self.$component_name.clear();)*
                }

                /// Remove all components for a given entity.
                #[allow(unused)]
                pub fn remove_entity(&mut self, entity: $crate::Entity) {
                    $(self.$component_name.remove(entity);)*
                }

                /// Clone each component of an entity into a `RealtimeEntityData`.
                #[allow(unused)]
                pub fn clone_entity_data(&self, entity: $crate::Entity) -> RealtimeEntityData {
                    RealtimeEntityData {
                        $($component_name: self.$component_name.get(entity).cloned(),)*
                    }
                }

                /// Remove each component of an entity into a `RealtimeEntityData`.
                #[allow(unused)]
                pub fn remove_entity_data(&mut self, entity: $crate::Entity) -> RealtimeEntityData {
                    RealtimeEntityData {
                        $($component_name: self.$component_name.remove(entity),)*
                    }
                }

                /// Insert each component in a `RealtimeEntityData` for an entity.
                #[allow(unused)]
                pub fn insert_entity_data(&mut self, entity: $crate::Entity, entity_data: RealtimeEntityData) {
                    $(if let Some(field) = entity_data.$component_name {
                        self.$component_name.insert(entity, field);
                    })*
                }

                /// Update all components of an entity to match a `RealtimeEntityData` (removing
                /// components that are absent from the `RealtimeEntityData`).
                #[allow(unused)]
                pub fn update_entity_data(&mut self, entity: $crate::Entity, entity_data: RealtimeEntityData) {
                    $(if let Some(field) = entity_data.$component_name {
                        self.$component_name.insert(entity, field);
                    } else {
                        self.$component_name.remove(entity);
                    })*
                }

                /// Tick the first component of an entity that is ready to be ticked within the
                /// remaining time. If no component can be ticked within the time frame, returns
                #[allow(unused)]
                pub fn tick_entity(
                    &mut self,
                    entity: $crate::Entity,
                    frame_remaining: std::time::Duration,
                ) -> (RealtimeEntityEvents, std::time::Duration) {
                    struct RealtimeEntityComponentsMut<'a> {
                        $($component_name: Option<&'a mut $crate::ScheduledRealtimeComponent<$component_type>>,)*
                    }
                    let mut components = RealtimeEntityComponentsMut {
                        $($component_name: self.$component_name.get_with_schedule_mut(entity),)*
                    };
                    let mut until_next_tick = frame_remaining;
                    $(if let Some(event) = components.$component_name.as_ref() {
                        until_next_tick = until_next_tick.min(event.until_next_tick);
                    })*
                    $(let $component_name = if let Some(scheduled_component) = components.$component_name.as_mut() {
                        if until_next_tick == scheduled_component.until_next_tick {
                            let (event, until_next_tick) = scheduled_component.component.tick();
                            scheduled_component.until_next_tick = until_next_tick;
                            Some(event)
                        } else {
                            scheduled_component.until_next_tick -= until_next_tick;
                            None
                        }
                    } else {
                        None
                    };)*
                    (RealtimeEntityEvents {
                        $($component_name,)*
                    }, until_next_tick)
                }
            }

            impl<$($lt,)*> $crate::RealtimeComponents<$context> for RealtimeComponents {
                type EntityEvents = RealtimeEntityEvents;

                fn tick_entity(
                    &mut self,
                    entity: Entity,
                    frame_remaining: Duration,
                ) -> (Self::EntityEvents, Duration) {
                    RealtimeComponents::tick_entity(self, entity, frame_remaining)
                }
            }
        }
    };
}
