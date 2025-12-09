// NOTE: Editor module is currently disabled for performance optimization.
// All code is preserved for future use. To re-enable:
// 1. Uncomment editor rendering in ui.rs
// 2. Uncomment /editor command handler in handlers.rs
// 3. Uncomment /editor in command hints in app.rs
// 4. Uncomment editor key handling in handlers.rs

#![allow(dead_code)] // Editor code kept for future use

pub mod editor;
pub mod editor_ui;
pub mod editor_handlers;

