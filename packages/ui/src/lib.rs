//! NAINA OS User Interface Framework package.

pub mod builder;
pub mod config;
pub mod error;
pub mod types;
pub mod ui_framework;

pub use builder::UIBuilder;
pub use config::UIConfig;
pub use error::{UIError, UIResult};
pub use types::{UIEvent, UIState, WindowRecord};
pub use ui_framework::UIFramework;
