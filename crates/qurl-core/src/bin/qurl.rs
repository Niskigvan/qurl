use anyhow::{anyhow, Result};
use async_std::{
    prelude::*,
    sync::{Arc, Mutex, MutexGuard},
    task,
};
use async_store::Store;
use qurl_core::{actions::AppAction, middlewares::tui::Tui, state::App};

fn main() -> Result<()> {
    // Create an application.
    task::block_on(async {
        let store = Arc::new(Mutex::new(Store::<App, AppAction>::default()));
        let _res = Tui::run(store.clone()).await;
    });

    // Initialize the terminal user interface.

    Ok(())
}
