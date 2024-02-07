// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::io::IsTerminal;

use once_cell::sync::Lazy;

#[cfg(feature = "colors")]
pub mod colors;

static IS_STDOUT_TTY: Lazy<bool> =
  Lazy::new(|| std::io::stdout().is_terminal());
static IS_STDERR_TTY: Lazy<bool> =
  Lazy::new(|| std::io::stderr().is_terminal());

pub fn is_stdout_tty() -> bool {
  *IS_STDOUT_TTY
}

pub fn is_stderr_tty() -> bool {
  *IS_STDERR_TTY
}
