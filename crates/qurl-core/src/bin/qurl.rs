use anyhow::{anyhow, Result};
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
    Increment,
}
#[derive(Debug, PartialEq, Clone)]
enum Event {
    IsOne,
    VChanged,
}

fn main() -> Result<()> {
    task::block_on(async {
        let store = Arc::new(Mutex::new(Store::<Counter, Action, Event>::default()));
        let s = store.clone();
        {
            let mut store = s.lock().await;
            store
                .on_action(move |state, action| {
                    Box::pin(async move {
                        let mut events = vec![];
                        match action {
                            _ => {
                                //future::ready(1)
                                let mut s = state.write().await;
                                s.v = (s.v + 1) % 100;
                                events.push(Event::VChanged)
                            }
                        }
                        DisposerResult {
                            state: state.clone(),
                            events,
                        }
                    })
                })
                .await;
            store
                .on_events(move |state, events| {
                    Box::pin(async move {
                        if events.iter().any(|e| e == &Event::VChanged) {
                            println!("{:?}", state.read().await);
                        }
                    })
                })
                .await;
        }
        loop {
            &store.dispatch(Action::Increment).await;
            println!("dispatch 1");
            &store.dispatch(Action::Increment).await;
            println!("dispatch 2");
            &store.dispatch(Action::Increment).await;
            println!("dispatch 3");
            &store.dispatch(Action::Increment).await;
            println!("dispatch 4");
            &store.dispatch(Action::Increment).await;
            println!("dispatch 5");
            &store.dispatch(Action::Increment).await;
            println!("dispatch 6");
            &store.dispatch(Action::Increment).await;
            println!("dispatch 7");
            &store.dispatch(Action::Increment).await;
            println!("dispatch 8");
            &store.dispatch(Action::Increment).await;
            println!("dispatch 9");
            &store.dispatch(Action::Increment).await;
            println!("dispatch 10");
            task::sleep(Duration::from_millis(5000)).await;
        }
        Ok(())
    })
}
