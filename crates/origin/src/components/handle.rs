use bevy::ecs::change_detection::MaybeLocation;

use tokio::task::{JoinError, JoinHandle};

#[derive(Debug)]
pub enum DbHandleError<E> {
    NotFinished,
    JoinError(JoinError),
    Error(E),
}

impl<E> DbHandleError<E> {
    pub fn is_finished(&self) -> bool {
        match self {
            DbHandleError::NotFinished => false,
            _ => true,
        }
    }
}

#[derive(Debug)]
pub struct DbHandleInner<O, T, E> {
    handle: JoinHandle<O>,
    data: Result<T, DbHandleError<E>>,
    caller: MaybeLocation,
}

pub type DbHandle<T, E = ()> = DbHandleInner<T, T, E>;

pub type DbHandleResult<T, E> = DbHandleInner<Result<T, E>, T, E>;

impl<O, T, E> DbHandleInner<O, T, E> {
    pub fn caller(&self) -> MaybeLocation {
        self.caller
    }

    pub fn handle(&self) -> &JoinHandle<O> {
        &self.handle
    }

    pub fn take_handle(self) -> JoinHandle<O> {
        self.handle
    }

    pub fn get_result(&self) -> Result<&T, &DbHandleError<E>> {
        self.data.as_ref()
    }

    pub fn get_result_mut(&mut self) -> Result<&mut T, &DbHandleError<E>> {
        self.data.as_mut().map_err(|err| &*err)
    }

    /// not any check,wrap!
    pub fn take_result(self) -> Result<T, DbHandleError<E>> {
        self.data
    }

    pub fn is_finished(&self) -> bool {
        match self.data.as_ref() {
            Ok(_) => true,
            Err(err) => err.is_finished(),
        }
    }
}

impl<T, E> DbHandleInner<T, T, E> {
    #[track_caller]
    pub fn new(handle: JoinHandle<T>) -> Self {
        Self {
            handle,
            data: Err(DbHandleError::NotFinished),
            caller: MaybeLocation::caller(),
        }
    }

    pub fn try_result(&mut self) -> Result<&mut T, &DbHandleError<E>> {
        let Self {
            handle,
            data,
            caller: _,
        } = self;

        if let Ok(data) = data {
            return Ok(data);
        }

        if !handle.is_finished() {
            return Err(&DbHandleError::NotFinished);
        }

        let result = bevy::tasks::block_on(handle).map_err(|err| DbHandleError::JoinError(err));

        *data = result;

        data.as_mut().map_err(|err| &*err)
    }

    pub fn block_on(&mut self) -> Result<&mut T, &DbHandleError<E>> {
        let Self {
            handle,
            data,
            caller: _,
        } = self;

        if !matches!(data, Err(DbHandleError::NotFinished)) {
            return data.as_mut().map_err(|err| &*err);
        }

        let result = bevy::tasks::block_on(handle).map_err(|err| DbHandleError::JoinError(err));

        *data = result;

        data.as_mut().map_err(|err| &*err)
    }
}

impl<T, E> DbHandleInner<Result<T, E>, T, E> {
    #[track_caller]
    pub fn new(handle: JoinHandle<Result<T, E>>) -> Self {
        Self {
            handle,
            data: Err(DbHandleError::NotFinished),
            caller: MaybeLocation::caller(),
        }
    }

    pub fn try_result(&mut self) -> Result<&mut T, &DbHandleError<E>> {
        let Self {
            handle,
            data,
            caller: _,
        } = self;

        if let Ok(data) = data {
            return Ok(data);
        }

        if !handle.is_finished() {
            return Err(&DbHandleError::NotFinished);
        }

        let result = bevy::tasks::block_on(handle).map_err(|err| DbHandleError::JoinError(err));

        *data = result.and_then(|result| result.map_err(|err| DbHandleError::Error(err)));

        data.as_mut().map_err(|err| &*err)
    }

    pub fn block_on(&mut self) -> Result<&mut T, &DbHandleError<E>> {
        let Self {
            handle,
            data,
            caller: _,
        } = self;

        if !matches!(data, Err(DbHandleError::NotFinished)) {
            return data.as_mut().map_err(|err| &*err);
        }

        let result = bevy::tasks::block_on(handle).map_err(|err| DbHandleError::JoinError(err));

        *data = result.and_then(|result| result.map_err(|err| DbHandleError::Error(err)));

        data.as_mut().map_err(|err: &mut DbHandleError<E>| &*err)
    }
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use tokio::runtime::Runtime;

    use super::*;

    #[test]
    fn block_on_many() {
        let runtime = Runtime::new().unwrap();

        let task = runtime.spawn(async {
            let task = tokio::spawn(async {
                tokio::time::sleep(Duration::from_secs_f32(1.0)).await;
                Result::<(), ()>::Ok(())
            });

            let mut handle = DbHandleInner::<Result<(), ()>, (), ()>::new(task);

            assert!(!handle.is_finished());

            let _ = handle.try_result();

            handle.block_on().unwrap();
            handle.block_on().unwrap();
            handle.block_on().unwrap();

            handle.get_result().unwrap();

            assert!(handle.is_finished());
            println!("handle size:{:?}", size_of_val(&handle));
        });

        runtime.block_on(task).unwrap()
    }
}
