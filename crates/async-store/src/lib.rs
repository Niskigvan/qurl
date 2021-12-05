use async_lock::Barrier;
use async_std::{
    future::Future,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
    task::{self, JoinHandle},
};
use std::{
    any::{Any, TypeId},
    pin::Pin,
    process::Output,
    time::Duration,
};
pub struct State<S> {
    state: Arc<RwLock<S>>,
}

pub trait ASAny: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Any> ASAny for T {
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
pub type ASAnyBoxed = Box<dyn Any + Send + Sync>; //Box<dyn ASAny + Send + Sync>;
pub enum Cond {
    _Changed(ASAnyBoxed),
    _Becomes(ASAnyBoxed, ASAnyBoxed),
}
impl Cond {
    #[allow(non_snake_case)]
    pub fn Changed<T: Sized + Any + Send + Sync>(val: T) -> Cond
    where
        T: PartialEq,
    {
        Cond::_Changed(Box::new(val))
    }
    #[allow(non_snake_case)]
    pub fn Becomes<T: Sized + Any + Send + Sync>(val: T, expectation: T) -> Cond
    where
        T: PartialEq,
    {
        Cond::_Becomes(Box::new(val), Box::new(expectation))
    }
}
/* trait CondCorrect {}
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
} */

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

/* pub trait IStore<Action, S>
where
    S: Any,
    Action: Send + Clone + 'static,
{
    fn dispatch(self, action: Action) -> JoinHandle<()>;
    fn effect<C, T>(&mut self, cond: C, effect: T) -> Box<dyn Future<Output = ()>>
    where
        C: 'static + Fn(RwLockReadGuard<S>) -> Cond + Send + Sync,
        T: 'static + Fn(ReadonlyState<S>) -> JoinHandle<Option<Vec<Action>>> + Send + Sync;
    fn on_action<T>(&mut self, listener: T) -> Box<dyn Future<Output = ()>>
    where
        T: 'static + Fn(State<S>, Action) -> JoinHandle<Option<Vec<Action>>> + Send + Sync;
    fn new(state: S) -> Arc<Mutex<Store<S, Action>>>;
} */
type ReactionsVec<S, A> = Vec<(
    Box<dyn Fn(RwLockReadGuard<S>) -> Box<dyn Any + Send + Sync> + Send + Sync>,
    Box<dyn Fn(ReadonlyState<S>) -> JoinHandle<Option<Vec<A>>> + Send + Sync>,
)>;
type HandlersVec<S, A> = Vec<Box<dyn Fn(State<S>, A) -> JoinHandle<Option<Vec<A>>> + Send + Sync>>;
pub struct Store<S, A>
where
    S: Any,
    A: Send + Clone,
{
    pub state: State<S>,
    reactions: Arc<RwLock<ReactionsVec<S, A>>>,
    handlers:
        Arc<RwLock<HandlersVec<S, A>>>,
    action: core::marker::PhantomData<A>,
}

impl<S, A> Store<S, A>
where
    S: Any,
    A: Send + Clone,
{
    pub async fn reaction<C, T>(&mut self, cond: C, effect: T)
    where
        C: 'static + Fn(RwLockReadGuard<S>) -> Cond + Send + Sync,
        T: 'static + Fn(ReadonlyState<S>) -> JoinHandle<Option<Vec<A>>> + Send + Sync,
    {
        self.reactions
            .write()
            .await
            .push((Box::new(cond), Box::new(effect)));
    }
    pub async fn handler<T>(&mut self, listener: T)
    where
        T: 'static + Fn(State<S>, A) -> JoinHandle<Option<Vec<A>>> + Send + Sync,
    {
        self.handlers.write().await.push(Box::new(listener))
    }
    /* pub async fn wait_until<C>(&mut self, cond: C)
    where
        C: 'static + Fn(RwLockReadGuard<S>) -> Cond + Send + Sync,
    {
        let barrier = Arc::new(Barrier::new(2));
        let barrier2 = barrier.clone();
        self.effect(cond, move |state| {
            let b = barrier2.clone();
            task::spawn(async move {
                b.wait().await;
                None
            })
        })
        .await;
        barrier.wait().await;
    } */
}
impl<S, A> Default for Store<S, A>
where
    S: Any + Default,
    A: Send + Clone,
{
    fn default() -> Self {
        Store::<S, A> {
            state: State::new(S::default()),
            reactions: Arc::new(RwLock::new(Vec::new())),
            handlers: Arc::new(RwLock::new(Vec::new())),
            action: core::marker::PhantomData,
        }
    }
}
pub trait ArcStore<S, A>
where
    S: Any + 'static,
    A: Send + Clone + 'static,
{
    fn do(self, action: A) -> JoinHandle<()>;
}
impl<S, A> ArcStore<S, A> for &Arc<Mutex<Store<S, A>>>
where
    S: Default + Sync + Send + 'static,
    A: Send + Clone + 'static,
{
    fn do(self, action: A) -> JoinHandle<()> {
        let store = self.clone();
        do(store, action)
    }
}

pub fn do<S, A>(store: Arc<Mutex<Store<S, A>>>, action: A) -> JoinHandle<()>
where
    S: Default + Sync + Send + 'static,
    A: Send + Clone + 'static,
{
    task::spawn(async move {
        let sl = store.lock().await;
        let disposers = sl.handlers.clone();
        let effects = sl.reactions.clone();
        let state = sl.state.clone();
        drop(sl);
        let mut actions: Vec<A> = vec![];
        let mut tasks: Vec<JoinHandle<Option<Vec<A>>>> = vec![];
        let mut eff_old: Vec<Cond> = vec![];

        for e in effects.read().await.iter() {
            eff_old.push(e.0(state.clone().read().await));
        }
        for r in disposers.read().await.iter() {
            let r = r(state.clone(), action.clone());
            tasks.push(r);
        }
        for t in tasks {
            let actions_ = t.await;
            if let Some(actions_) = actions_ {
                actions.extend(actions_);
            }
        }

        let mut tasks: Vec<JoinHandle<Option<Vec<A>>>> = vec![];
        for (i, e) in effects.read().await.iter().enumerate() {
            let v_new = e.0(state.clone().read().await);
            match eff_old.get(i) {
                Some(Cond::_Becomes(v_old, ex)) => match v_new {
                    Cond::_Becomes(v_new, ex) => {
                        let _ex = ex.as_any().downcast_ref::<bool>();
                        let _v = v_new.as_any().downcast_ref::<bool>();
                        let _o = v_old.as_any().downcast_ref::<bool>();
                        if _ex.is_some()
                            && _o.is_some()
                            && _v.is_some()
                            && _o.unwrap() != _ex.unwrap()
                            && _v.unwrap() == _ex.unwrap()
                        {
                            tasks.push(e.1(state.clone().into()));
                        }
                        /* match _v {
                            Some(&true) => match _o {
                                Some(&false) => {
                                    tasks.push(ef(state.clone().into()));
                                }
                                _ => {}
                            },
                            _ => {}
                        } */
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        for t in tasks {
            let actions_ = t.await;
            if let Some(actions_) = actions_ {
                actions.extend(actions_);
            }
        }
        let mut tasks: Vec<JoinHandle<()>> = vec![];
        for action in actions {
            tasks.push(do(store.clone(), action));
        }
    })
}
