#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![doc = include_str!("../README.md")]

pub mod cmd;

pub use cmd::{Cmd, CmdBuilder, Error};
pub use shx_macros::lex;

/// `Result` from std, with the error type defaulting to shx's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Similar to the lower-level [`lex`] macro that only lex one command.
#[macro_export]
macro_rules! cmd {
    ($($stream:tt)+) => {
        (
            $crate::lex!($($stream)*).next().expect("Need one command at least")
        )
    };
}

/// Similar to the lower-level [`lex`] macro that also executes the commands in order.
#[macro_export]
macro_rules! shx {
    ($($stream:tt)*) => {
        (
            $crate::lex!($($stream)*)
                .map(|mut cmd| cmd.exec())
                .collect::<Result<Vec<_>, _>>()
                .map(|_| ())
        )
    };
}
