use std::mem;

use bevy::ecs::change_detection::MaybeLocation;

use tokio::task::{JoinError, JoinHandle};

#[derive(Debug)]
pub enum ECSHandleError<E> {
    NotFinished,
    HasBeenTake,
    JoinError(JoinError),
    Error(E),
}

impl<E> ECSHandleError<E> {
    pub fn is_finished(&self) -> bool {
        match self {
            ECSHandleError::NotFinished => false,
            _ => true,
        }
    }
}

#[derive(Debug)]
pub struct ECSHandleInner<O, T, E> {
    handle: JoinHandle<O>,
    data: Result<T, ECSHandleError<E>>,
    caller: MaybeLocation,
}

pub type ECSHandle<T, E = ()> = ECSHandleInner<T, T, E>;

pub type ECSHandleResult<T, E> = ECSHandleInner<Result<T, E>, T, E>;

impl<O, T, E> ECSHandleInner<O, T, E> {
    pub fn caller(&self) -> MaybeLocation {
        self.caller
    }

    pub fn handle(&self) -> &JoinHandle<O> {
        &self.handle
    }

    pub fn take_handle(self) -> JoinHandle<O> {
        self.handle
    }

    pub fn get_result(&self) -> Result<&T, &ECSHandleError<E>> {
        self.data.as_ref()
    }

    pub fn get_result_mut(&mut self) -> Result<&mut T, &ECSHandleError<E>> {
        self.data.as_mut().map_err(|err| &*err)
    }

    /// not any check,wrap!
    pub fn as_result(self) -> Result<T, ECSHandleError<E>> {
        self.data
    }

    /// not any check,wrap!
    pub fn take_result(&mut self) -> Result<T, ECSHandleError<E>> {
        mem::replace(&mut self.data, Err(ECSHandleError::HasBeenTake))
    }

    pub fn is_finished(&self) -> bool {
        match self.data.as_ref() {
            Ok(_) => true,
            Err(err) => err.is_finished() || self.handle.is_finished(),
        }
    }

    #[track_caller]
    pub fn repeat(&mut self, src: JoinHandle<O>) -> Self {
        let Self {
            handle,
            data,
            caller,
        } = self;

        let handle = mem::replace(handle, src);
        let data = mem::replace(data, Err(ECSHandleError::NotFinished));
        let caller = mem::replace(caller, MaybeLocation::caller());

        Self {
            handle,
            data,
            caller,
        }
    }
}

impl<T, E> ECSHandleInner<T, T, E> {
    #[track_caller]
    pub fn new(handle: JoinHandle<T>) -> Self {
        Self {
            handle,
            data: Err(ECSHandleError::NotFinished),
            caller: MaybeLocation::caller(),
        }
    }

    pub fn try_result(&mut self) -> Result<&mut T, &ECSHandleError<E>> {
        let Self {
            handle,
            data,
            caller: _,
        } = self;

        if let Ok(data) = data {
            return Ok(data);
        }

        if !handle.is_finished() {
            return Err(&ECSHandleError::NotFinished);
        }

        let result = bevy::tasks::block_on(handle).map_err(|err| ECSHandleError::JoinError(err));

        *data = result;

        data.as_mut().map_err(|err| &*err)
    }

    pub fn block_on(&mut self) -> Result<&mut T, &ECSHandleError<E>> {
        let Self {
            handle,
            data,
            caller: _,
        } = self;

        if !matches!(data, Err(ECSHandleError::NotFinished)) {
            return data.as_mut().map_err(|err| &*err);
        }

        let result = bevy::tasks::block_on(handle).map_err(|err| ECSHandleError::JoinError(err));

        *data = result;

        data.as_mut().map_err(|err| &*err)
    }

    pub fn block_on_take_result(mut self) -> Result<T, ECSHandleError<E>> {
        let _ = self.block_on();
        self.data
    }
}

impl<T, E> ECSHandleInner<Result<T, E>, T, E> {
    #[track_caller]
    pub fn new(handle: JoinHandle<Result<T, E>>) -> Self {
        Self {
            handle,
            data: Err(ECSHandleError::NotFinished),
            caller: MaybeLocation::caller(),
        }
    }

    pub fn try_result(&mut self) -> Result<&mut T, &ECSHandleError<E>> {
        let Self {
            handle,
            data,
            caller: _,
        } = self;

        if let Ok(data) = data {
            return Ok(data);
        }

        if !handle.is_finished() {
            return Err(&ECSHandleError::NotFinished);
        }

        let result = bevy::tasks::block_on(handle).map_err(|err| ECSHandleError::JoinError(err));

        *data = result.and_then(|result| result.map_err(|err| ECSHandleError::Error(err)));

        data.as_mut().map_err(|err| &*err)
    }

    pub fn block_on(&mut self) -> Result<&mut T, &ECSHandleError<E>> {
        let Self {
            handle,
            data,
            caller: _,
        } = self;

        if !matches!(data, Err(ECSHandleError::NotFinished)) {
            return data.as_mut().map_err(|err| &*err);
        }

        let result = bevy::tasks::block_on(handle).map_err(|err| ECSHandleError::JoinError(err));

        *data = result.and_then(|result| result.map_err(|err| ECSHandleError::Error(err)));

        data.as_mut().map_err(|err: &mut ECSHandleError<E>| &*err)
    }

    pub fn block_on_take_result(mut self) -> Result<T, ECSHandleError<E>> {
        let _ = self.block_on();
        self.data
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

            let mut handle = ECSHandleInner::<Result<(), ()>, (), ()>::new(task);

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
