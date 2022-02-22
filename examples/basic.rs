use serde::{Deserialize, Serialize};

use entity_table_realtime::{
    declare_realtime_entity_module, Entity, RealtimeComponent, RealtimeComponentApplyEvent,
};
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dummy;

impl RealtimeComponent for Dummy {
    type Event = ();

    fn tick(&mut self) -> (Self::Event, Duration) {
        ((), Duration::from_millis(0))
    }
}

impl RealtimeComponentApplyEvent<()> for Dummy {
    fn apply_event(_: <Self as RealtimeComponent>::Event, _: Entity, _: &mut ()) {}
}

declare_realtime_entity_module! {
    components_no_lifetime[()] {
        dummy: Dummy,
    }
}

pub struct Context1<'a>(&'a mut ());
impl<'a> RealtimeComponentApplyEvent<Context1<'a>> for Dummy {
    fn apply_event(_: <Self as RealtimeComponent>::Event, _: Entity, _: &mut Context1<'a>) {}
}

declare_realtime_entity_module! {
    components_one_lifetime<'a>[Context1<'a>] {
        dummy: Dummy,
    }
}

pub struct Context2<'a, 'b>(&'a mut (), &'b mut ());
impl<'a, 'b> RealtimeComponentApplyEvent<Context2<'a, 'b>> for Dummy {
    fn apply_event(_: <Self as RealtimeComponent>::Event, _: Entity, _: &mut Context2<'a, 'b>) {}
}

declare_realtime_entity_module! {
    components_two_lifetimes<'a, 'b>[Context2<'a, 'b>] {
        dummy: Dummy,
    }
}

fn main() {}
