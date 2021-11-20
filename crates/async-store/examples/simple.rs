use async_std::{
    future,
    io::{prelude::BufReadExt, Lines},
    prelude::*,
    sync::{Arc, Mutex, MutexGuard},
    task,
};
use async_store::*;
use std::{borrow::Borrow, process::Output, time::Duration};

#[derive(Default, Debug, PartialEq)]
struct Counter {
    v: u32,
}
#[derive(Debug, PartialEq, Clone)]
enum Action {
    Increment(u32),
}

fn main() {
    task::block_on(async {
        let store = Arc::new(Mutex::new(Store::<Counter, Action>::default()));
        store
            .lock()
            .await
            .on_action(move |state, action| {
                Box::pin(async move {
                    match action {
                        Action::Increment(v) => {
                            //future::ready(1)
                            let mut s = state.write().await;
                            s.v = (s.v + v) % 1000;
                            //task::sleep(Duration::from_millis(500)).await;
                            //s2.dispatch(Action::Increment(1)).await;
                        }
                        _ => {}
                    }
                    state.clone()
                })
            })
            .await;

        store
            .lock()
            .await
            .effect(
                |state| Box::new(EqAnyW(state.v > 0 && state.v % 23 == 0)),
                move |state| {
                    Box::pin(async move {
                        println!("{:?}  [v%23==0]", state.read().await);
                        store.dispatch(Action::Increment(1)).await;
                    })
                },
            )
            .await;
        loop {
            store.dispatch(Action::Increment(1)).await;
            store.dispatch(Action::Increment(1)).await;
            store.dispatch(Action::Increment(1)).await;
            store.dispatch(Action::Increment(1)).await;
            store.dispatch(Action::Increment(1)).await;
            store.dispatch(Action::Increment(1)).await;
            store.dispatch(Action::Increment(1)).await;
            store.dispatch(Action::Increment(1)).await;
            store.dispatch(Action::Increment(1)).await;
            store.dispatch(Action::Increment(1)).await;
            task::sleep(Duration::from_secs(2)).await;
        }
    })
}
