//! Rust library for managing your personal information ([PIM]).
//!
//! Pimalaya is a Rust library that gathers logic related to [PIM] in
//! order to manage your personal information: send an email, list
//! your contacts, reply to an event etc.
//!
//! This project serves as a basement for all kind of top-level
//! applications: CLI, TUI, GUI, plugins, servers etc.
//!
//! The fact that the domain logic is separated from interfaces makes
//! this project very flexible. You can build the interface of your
//! dream without reinventing the wheel.
//!
//! [PIM]: https://en.wikipedia.org/wiki/Personal_information_manager

pub mod time;
