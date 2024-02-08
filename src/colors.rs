// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use once_cell::sync::Lazy;
use std::fmt;
use std::fmt::Write as _;
use std::sync::atomic::AtomicBool;
use termcolor::Ansi;
use termcolor::Color::Ansi256;
use termcolor::Color::Black;
use termcolor::Color::Blue;
use termcolor::Color::Cyan;
use termcolor::Color::Green;
use termcolor::Color::Magenta;
use termcolor::Color::Red;
use termcolor::Color::White;
use termcolor::Color::Yellow;
use termcolor::ColorSpec;
use termcolor::WriteColor;

#[cfg(windows)]
use termcolor::BufferWriter;
#[cfg(windows)]
use termcolor::ColorChoice;

static USE_COLOR: Lazy<AtomicBool> = Lazy::new(|| {
  #[cfg(wasm)]
  {
    // Don't use color by default on Wasm targets.
    // Instead the user can opt-in via `set_use_color`.
    AtomicBool::new(false)
  }
  #[cfg(not(wasm))]
  {
    let no_color = std::env::var_os("NO_COLOR")
      .map(|v| !v.is_empty())
      .unwrap_or(false);
    AtomicBool::new(!no_color)
  }
});

/// Gets whether color should be used in the output.
///
/// This is informed via the `USE_COLOR` environment variable
/// or if `set_use_color` has been set to true.
///
/// On Wasm targets, use `set_use_color(true)` to enable color output.
pub fn use_color() -> bool {
  USE_COLOR.load(std::sync::atomic::Ordering::Relaxed)
}

/// Sets whether color should be used in the output.
///
/// This overrides the default value set via the `NO_COLOR` env var.
pub fn set_use_color(use_color: bool) {
  USE_COLOR.store(use_color, std::sync::atomic::Ordering::Relaxed);
}

/// Enables ANSI color output on Windows. This is a no-op on other platforms.
pub fn enable_ansi() {
  #[cfg(windows)]
  {
    BufferWriter::stdout(ColorChoice::AlwaysAnsi);
  }
}

/// A struct that can adapt a `fmt::Write` to a `std::io::Write`. If anything
/// that can not be represented as UTF-8 is written to this writer, it will
/// return an error.
struct StdFmtStdIoWriter<'a>(&'a mut dyn fmt::Write);

impl std::io::Write for StdFmtStdIoWriter<'_> {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    let str = std::str::from_utf8(buf).map_err(|_| {
      std::io::Error::new(
        std::io::ErrorKind::Other,
        "failed to convert bytes to str",
      )
    })?;
    match self.0.write_str(str) {
      Ok(_) => Ok(buf.len()),
      Err(_) => Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "failed to write to fmt::Write",
      )),
    }
  }

  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}

/// A struct that can adapt a `std::io::Write` to a `fmt::Write`.
struct StdIoStdFmtWriter<'a>(&'a mut dyn std::io::Write);

impl fmt::Write for StdIoStdFmtWriter<'_> {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    self.0.write_all(s.as_bytes()).map_err(|_| fmt::Error)?;
    Ok(())
  }
}

pub struct Style<I: fmt::Display> {
  colorspec: ColorSpec,
  inner: I,
}

impl<I: fmt::Display> fmt::Display for Style<I> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if !use_color() {
      return fmt::Display::fmt(&self.inner, f);
    }
    let mut ansi_writer = Ansi::new(StdFmtStdIoWriter(f));
    ansi_writer
      .set_color(&self.colorspec)
      .map_err(|_| fmt::Error)?;
    write!(StdIoStdFmtWriter(&mut ansi_writer), "{}", self.inner)?;
    ansi_writer.reset().map_err(|_| fmt::Error)?;
    Ok(())
  }
}

#[inline]
fn style<'a, S: fmt::Display + 'a>(s: S, colorspec: ColorSpec) -> Style<S> {
  Style {
    colorspec,
    inner: s,
  }
}

pub fn red_bold<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Red)).set_bold(true);
  style(s, style_spec)
}

pub fn green_bold<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Green)).set_bold(true);
  style(s, style_spec)
}

pub fn yellow_bold<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Yellow)).set_bold(true);
  style(s, style_spec)
}

pub fn italic<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_italic(true);
  style(s, style_spec)
}

pub fn italic_gray<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Ansi256(8))).set_italic(true);
  style(s, style_spec)
}

pub fn italic_bold<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_bold(true).set_italic(true);
  style(s, style_spec)
}

pub fn white_on_red<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_bg(Some(Red)).set_fg(Some(White));
  style(s, style_spec)
}

pub fn black_on_green<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_bg(Some(Green)).set_fg(Some(Black));
  style(s, style_spec)
}

pub fn yellow<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Yellow));
  style(s, style_spec)
}

pub fn cyan<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Cyan));
  style(s, style_spec)
}

pub fn cyan_with_underline<'a>(
  s: impl fmt::Display + 'a,
) -> impl fmt::Display + 'a {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Cyan)).set_underline(true);
  style(s, style_spec)
}

pub fn cyan_bold<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Cyan)).set_bold(true);
  style(s, style_spec)
}

pub fn magenta<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Magenta));
  style(s, style_spec)
}

pub fn red<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Red));
  style(s, style_spec)
}

pub fn green<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Green));
  style(s, style_spec)
}

pub fn bold<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_bold(true);
  style(s, style_spec)
}

pub fn gray<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Ansi256(245)));
  style(s, style_spec)
}

pub fn intense_blue<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Blue)).set_intense(true);
  style(s, style_spec)
}

pub fn white_bold_on_red<'a>(
  s: impl fmt::Display + 'a,
) -> impl fmt::Display + 'a {
  let mut style_spec = ColorSpec::new();
  style_spec
    .set_bold(true)
    .set_bg(Some(Red))
    .set_fg(Some(White));
  style(s, style_spec)
}
