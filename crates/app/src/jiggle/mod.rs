//! The jiggler: when to move, how to move, and when to let go.
//!
//! - `jiggler`: the state machine, a port of the macOS app's `Jiggler.swift`.
//! - `pointer`: reads and moves the pointer, reads the idle time and the buttons.
//! - `watch`: raw input, to notice keys, buttons and the wheel during a stroll.
//! - `session`: whether anyone can be at this session's screen.
//!
//! Safety rule: MelCat only ever *moves* the pointer. It never installs a hook
//! that could hold or swallow input, so whatever goes wrong, the user's own
//! mouse and keyboard keep working.

pub(crate) mod jiggler;
pub(crate) mod pointer;
pub(crate) mod session;
pub(crate) mod watch;

pub(crate) use jiggler::Status;
