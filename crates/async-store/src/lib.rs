use async_std::{
    future::Future,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
    task::{self, JoinHandle},
};
use std::{any::Any, pin::Pin, process::Output};
pub struct State<S> {
    state: Arc<RwLock<S>>,
}

pub trait ASAny: Any + PartialEq<dyn Any> {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Any + PartialEq<dyn Any>> ASAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// This wrapper implements PartialEq<dyn Any> which is needed to
/// Allow MyAny to become a trait object
///

// pub enum Cond<T: ?Sized> {
//     Changed(T),
//     BecomesTrue(bool),
// }

pub struct EqAnyW<T: ?Sized>(pub T);

impl<T: 'static> PartialEq<dyn Any> for EqAnyW<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &dyn Any) -> bool {
        match other.downcast_ref::<T>() {
            Some(a) => &self.0 == a,
            None => false,
        }
    }
}

impl<S> State<S> {
    pub fn new(s: S) -> Self {
        State {
            state: Arc::new(RwLock::new(s)),
        }
    }
    pub fn read(&self) -> impl Future<Output = RwLockReadGuard<S>> {
        self.state.read()
    }
    pub fn write(&self) -> impl Future<Output = RwLockWriteGuard<S>> {
        self.state.write()
    }
}

impl<S> Clone for State<S> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

pub struct ReadonlyState<S>(State<S>);
impl<S> ReadonlyState<S> {
    pub fn read(&self) -> impl Future<Output = RwLockReadGuard<S>> {
        self.0.read()
    }
}
impl<S> Clone for ReadonlyState<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<S> From<State<S>> for ReadonlyState<S> {
    fn from(s: State<S>) -> Self {
        ReadonlyState(s)
    }
}

pub type DisposerFn<S, A> =
    fn(State<S>, A) -> Pin<Box<dyn Future<Output = State<S>> + Send + Sync>>;

pub struct Store<S, A>
where
    S: Default + Sync + Send,
    A: Send + Clone,
{
    pub state: State<S>,
    effects: Arc<
        RwLock<
            Vec<(
                Box<dyn Fn(RwLockReadGuard<S>) -> Box<dyn ASAny + Send + Sync> + Send + Sync>,
                Box<
                    dyn Fn(ReadonlyState<S>) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>>
                        + Send
                        + Sync,
                >,
            )>,
        >,
    >,
    disposers: Arc<
        RwLock<
            Vec<
                Box<
                    dyn Fn(State<S>, A) -> Pin<Box<dyn Future<Output = State<S>> + Send + Sync>>
                        + Send
                        + Sync,
                >,
            >,
        >,
    >,
    action: core::marker::PhantomData<A>,
}

impl<S, A> Store<S, A>
where
    S: Default + Sync + Send,
    A: Send + Clone,
{
    pub async fn effect<C, T>(&mut self, cond: C, effect: T)
    where
        C: 'static + Fn(RwLockReadGuard<S>) -> Box<dyn ASAny + Send + Sync> + Send + Sync,
        T: 'static
            + Fn(ReadonlyState<S>) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>>
            + Send
            + Sync,
    {
        self.effects
            .write()
            .await
            .push((Box::new(cond), Box::new(effect)));
    }
    pub async fn on_action<T>(&mut self, listener: T)
    where
        T: 'static
            + Fn(State<S>, A) -> Pin<Box<dyn Future<Output = State<S>> + Send + Sync>>
            + Send
            + Sync,
    {
        self.disposers.write().await.push(Box::new(listener))
    }
}
pub trait ArcStore<S, A>
where
    S: Default + Sync + Send + 'static,
    A: Send + Clone + 'static,
{
    fn dispatch(self, action: A) -> Pin<Box<dyn Future<Output = JoinHandle<()>> + Send + Sync>>;
}
impl<S, A> ArcStore<S, A> for &Arc<Mutex<Store<S, A>>>
where
    S: Default + Sync + Send + 'static,
    A: Send + Clone + 'static,
{
    fn dispatch(self, action: A) -> Pin<Box<dyn Future<Output = JoinHandle<()>> + Send + Sync>> {
        let store = self.clone();
        Box::pin(async move {
            let sl = store.lock().await;
            let disposers = sl.disposers.clone();
            let effects = sl.effects.clone();
            let state = sl.state.clone();
            drop(sl);
            task::spawn(async move {
                let mut tasks: Vec<JoinHandle<State<S>>> = vec![];
                let mut eff_old: Vec<Box<dyn ASAny + Send + Sync>> = vec![];

                for e in effects.read().await.iter() {
                    eff_old.push(e.0(state.clone().read().await));
                }
                for r in disposers.read().await.iter() {
                    let r = r(state.clone(), action.clone());
                    tasks.push(task::spawn(async move { r.await }));
                }
                for t in tasks {
                    t.await;
                }

                let mut tasks: Vec<JoinHandle<()>> = vec![];
                for e in effects.read().await.iter() {
                    for v_old in eff_old.iter() {
                        let v_new: Box<dyn ASAny + Send + Sync> = e.0(state.clone().read().await);
                        let _v = v_new.as_any().downcast_ref::<EqAnyW<u32>>();
                        let _o = v_old.as_any().downcast_ref::<EqAnyW<u32>>();

                        if _v.is_some() && _o.is_some() && _v.unwrap().0 != _o.unwrap().0 {
                            let e = e.1(state.clone().into());
                            tasks.push(task::spawn(async move { e.await }));
                        }
                        let _v = v_new.as_any().downcast_ref::<EqAnyW<bool>>();
                        let _o = v_old.as_any().downcast_ref::<EqAnyW<bool>>();

                        if _v.is_some()
                            && _o.is_some()
                            && _v.unwrap().0 == true
                            && _o.unwrap().0 == false
                        {
                            let e = e.1(state.clone().into());
                            tasks.push(task::spawn(async move { e.await }));
                        }
                    }
                }
                for t in tasks {
                    t.await;
                }
            })
        })
    }
}

impl<S, A> Default for Store<S, A>
where
    S: Default + Sync + Send,
    A: Send + Clone,
{
    fn default() -> Self {
        Store::<S, A> {
            state: State::new(S::default()),
            effects: Arc::new(RwLock::new(Vec::new())),
            disposers: Arc::new(RwLock::new(Vec::new())),
            action: core::marker::PhantomData,
        }
    }
}
