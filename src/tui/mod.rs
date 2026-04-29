//! Terminal user interface: owns terminal setup/teardown and the event loop.

pub mod app;
pub mod run;
pub mod theme;
pub mod ui;

pub use run::run;
