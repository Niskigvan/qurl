use async_lock::Barrier;
use async_std::{
    future::Future,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
    task::{self, JoinHandle},
};
use std::{any::Any, pin::Pin, process::Output, time::Duration};
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

pub enum Cond {
    _Changed(Box<dyn ASAny + Send + Sync>),
    _BecomesTrue(Box<dyn ASAny + Send + Sync>),
    _BecomesFalse(Box<dyn ASAny + Send + Sync>),
}
impl Cond {
    /* pub fn Changed<T: 'static>(v: T) -> Cond
    where
        T: PartialEq,
    {
        Cond::_Changed(Box::new(EqAnyW(v)))
    } */
    pub fn becomes_true(v: bool) -> Cond {
        Cond::_BecomesTrue(Box::new(EqAnyW(v)))
    }
    pub fn becomes_false(v: bool) -> Cond {
        Cond::_BecomesFalse(Box::new(EqAnyW(v)))
    }
}
trait CondCorrect {}
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

pub trait IStore<Action, S>
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
}

pub struct Store<S, A>
where
    S: Any,
    A: Send + Clone,
{
    pub state: State<S>,
    effects: Arc<
        RwLock<
            Vec<(
                Box<dyn Fn(RwLockReadGuard<S>) -> Cond + Send + Sync>,
                Box<dyn Fn(ReadonlyState<S>) -> JoinHandle<Option<Vec<A>>> + Send + Sync>,
            )>,
        >,
    >,
    disposers:
        Arc<RwLock<Vec<Box<dyn Fn(State<S>, A) -> JoinHandle<Option<Vec<A>>> + Send + Sync>>>>,
    action: core::marker::PhantomData<A>,
}

impl<S, A> Store<S, A>
where
    S: Any,
    A: Send + Clone,
{
    pub async fn effect<C, T>(&mut self, cond: C, effect: T)
    where
        C: 'static + Fn(RwLockReadGuard<S>) -> Cond + Send + Sync,
        T: 'static + Fn(ReadonlyState<S>) -> JoinHandle<Option<Vec<A>>> + Send + Sync,
    {
        self.effects
            .write()
            .await
            .push((Box::new(cond), Box::new(effect)));
    }
    pub async fn on_action<T>(&mut self, listener: T)
    where
        T: 'static + Fn(State<S>, A) -> JoinHandle<Option<Vec<A>>> + Send + Sync,
    {
        self.disposers.write().await.push(Box::new(listener))
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
            effects: Arc::new(RwLock::new(Vec::new())),
            disposers: Arc::new(RwLock::new(Vec::new())),
            action: core::marker::PhantomData,
        }
    }
}
pub trait ArcStore<S, A>
where
    S: Any + 'static,
    A: Send + Clone + 'static,
{
    fn dispatch(self, action: A) -> JoinHandle<()>;
}
impl<S, A> ArcStore<S, A> for &Arc<Mutex<Store<S, A>>>
where
    S: Default + Sync + Send + 'static,
    A: Send + Clone + 'static,
{
    fn dispatch(self, action: A) -> JoinHandle<()> {
        let store = self.clone();
        dispatch(store, action)
    }
}

pub fn dispatch<S, A>(store: Arc<Mutex<Store<S, A>>>, action: A) -> JoinHandle<()>
where
    S: Default + Sync + Send + 'static,
    A: Send + Clone + 'static,
{
    task::spawn(async move {
        let sl = store.lock().await;
        let disposers = sl.disposers.clone();
        let effects = sl.effects.clone();
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
            let v_new: Cond = e.0(state.clone().read().await);
            match eff_old.get(i) {
                Some(Cond::_BecomesTrue(v_old)) => match v_new {
                    Cond::_BecomesTrue(v_new) => {
                        let _v = v_new.as_any().downcast_ref::<EqAnyW<bool>>();
                        let _o = v_old.as_any().downcast_ref::<EqAnyW<bool>>();
                        match _v {
                            Some(&EqAnyW(true)) => match _o {
                                Some(&EqAnyW(false)) => {
                                    tasks.push(e.1(state.clone().into()));
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                    _ => {}
                },
                Some(Cond::_BecomesFalse(v_old)) => match v_new {
                    Cond::_BecomesFalse(v_new) => {
                        let _v = v_new.as_any().downcast_ref::<EqAnyW<bool>>();
                        let _o = v_old.as_any().downcast_ref::<EqAnyW<bool>>();
                        match _v {
                            Some(&EqAnyW(false)) => match _o {
                                Some(&EqAnyW(true)) => {
                                    tasks.push(e.1(state.clone().into()));
                                }
                                _ => {}
                            },
                            _ => {}
                        }
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
            tasks.push(dispatch(store.clone(), action));
        }
    })
}
