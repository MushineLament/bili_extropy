use bevy::ecs::{
    component::Component,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, IntoSystem, Res, ResMut, System},
};
use bevy_tokio_tasks::TokioTasksRuntime;

use crate::db::Db;

pub trait DbInitailizeComponent {
    fn to_system(
        self,
    ) -> bevy::ecs::schedule::ScheduleConfigs<Box<dyn System<In = (), Out = ()> + 'static>>;
}

impl<F, O> DbInitailizeComponent for F
where
    F: Fn(Db, &mut TokioTasksRuntime) -> O + Send + Sync + 'static,
    O: Component,
{
    fn to_system(
        self,
    ) -> bevy::ecs::schedule::ScheduleConfigs<Box<dyn System<In = (), Out = ()> + 'static>> {
        let func = self;
        let system = IntoSystem::into_system(
            move |mut commands: Commands, db: Res<Db>, mut runtimer: ResMut<TokioTasksRuntime>| {
                let result = func(db.clone(), runtimer.as_mut());
                commands.spawn(result);
            },
        );

        system.into_configs()
    }
}

pub trait DbInitailizeResource {
    fn to_system(
        self,
    ) -> bevy::ecs::schedule::ScheduleConfigs<Box<dyn System<In = (), Out = ()> + 'static>>;
}

impl<F, O> DbInitailizeResource for F
where
    F: Fn(Db, &mut TokioTasksRuntime) -> O + Send + Sync + 'static,
    O: Resource,
{
    fn to_system(
        self,
    ) -> bevy::ecs::schedule::ScheduleConfigs<Box<dyn System<In = (), Out = ()> + 'static>> {
        let func = self;
        let system = IntoSystem::into_system(
            move |mut commands: Commands, db: Res<Db>, mut runtimer: ResMut<TokioTasksRuntime>| {
                let result = func(db.clone(), runtimer.as_mut());
                commands.insert_resource(result);
            },
        );

        system.into_configs()
    }
}
