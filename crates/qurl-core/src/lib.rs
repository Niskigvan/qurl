use async_std::sync::{Arc, Mutex, MutexGuard};
pub mod actions;
pub mod middlewares;
pub mod state;
use actions::AppAction;
use async_store::Store;
use state::App;
pub type AppStore = Arc<Mutex<Store<App, AppAction>>>;
