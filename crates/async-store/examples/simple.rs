use async_std::{
    future,
    io::{prelude::BufReadExt, Lines},
    prelude::*,
    sync::{Arc, Mutex, MutexGuard},
    task,
};
use async_store::*;
use std::time::Duration;

#[derive(Default, Debug, PartialEq)]
struct Counter {
    v: u32,
}
#[derive(Debug, PartialEq, Clone)]
enum Action {
    Increment(i32),
}

fn main() {
    task::block_on(async {
        let store = Arc::new(Mutex::new(Store::<Counter, Action>::default()));
        store
            .lock()
            .await
            .on_action(move |state, action| {
                task::spawn(async move {
                    match action {
                        Action::Increment(v) => {
                            //future::ready(1)
                            let mut s = state.write().await;
                            s.v = ((s.v as i32 + v).max(0) as u32) % 1000;
                            //task::sleep(Duration::from_millis(500)).await;
                            //s2.dispatch(Action::Increment(1)).await;
                        }
                    }
                    None
                })
            })
            .await;
        store
            .lock()
            .await
            .effect(
                |state| Cond::becomes_true(state.v > 0 && state.v % 23 == 0),
                move |state| {
                    task::spawn(async move {
                        println!("{:?}  [v%23==0]", state.read().await);
                        //store.dispatch(Action::Increment(1));

                        Some(vec![Action::Increment(-(state.read().await.v as i32))])
                    })
                },
            )
            .await;
        loop {
            store.dispatch(Action::Increment(1));

            task::sleep(Duration::from_micros(8)).await;
        }
    })
}
