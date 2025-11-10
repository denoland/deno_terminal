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

static FORCE_COLOR: Lazy<bool> = Lazy::new(|| {
  std::env::var_os("FORCE_COLOR")
    .map(|v| !v.is_empty())
    .unwrap_or(false)
});

static USE_COLOR: Lazy<AtomicBool> = Lazy::new(|| {
  #[cfg(wasm)]
  {
    // Don't use color by default on Wasm targets because
    // it's not always possible to read the `NO_COLOR` env var.
    //
    // Instead the user can opt-in via `set_use_color`.
    AtomicBool::new(false)
  }
  #[cfg(not(wasm))]
  {
    if *FORCE_COLOR {
      return AtomicBool::new(true);
    }

    let no_color = std::env::var_os("NO_COLOR")
      .map(|v| !v.is_empty())
      .unwrap_or(false);
    AtomicBool::new(!no_color)
  }
});

#[derive(Clone, Copy, Debug)]
pub enum ColorLevel {
  None,
  Ansi,
  Ansi256,
  TrueColor,
}

static COLOR_LEVEL: Lazy<ColorLevel> = Lazy::new(|| {
  #[cfg(wasm)]
  {
    // Don't use color by default on Wasm targets because
    // it's not always possible to read env vars.
    ColorLevel::None
  }
  #[cfg(not(wasm))]
  {
    // If FORCE_COLOR is not set, check for NO_COLOR
    if !*FORCE_COLOR {
      let no_color = std::env::var_os("NO_COLOR")
        .map(|v| !v.is_empty())
        .unwrap_or(false);

      if no_color {
        return ColorLevel::None;
      }
    }

    let mut term = "dumb".to_string();
    if let Some(term_value) = std::env::var_os("TERM") {
      // If FORCE_COLOR is set, ignore the "dumb" terminal check
      if term_value == "dumb" && !*FORCE_COLOR {
        return ColorLevel::None;
      }

      if let Some(term_value) = term_value.to_str() {
        term = term_value.to_string();
      }
    }

    #[cfg(platform = "windows")]
    {
      // TODO: Older versions of windows only support ansi colors,
      // starting with Windows 10 build 10586 ansi256 is supported

      // TrueColor support landed with Windows 10 build 14931,
      // see https://devblogs.microsoft.com/commandline/24-bit-color-in-the-windows-console/
      return ColorLevel::TrueColor;
    }

    if std::env::var_os("TMUX").is_some() {
      return ColorLevel::Ansi;
    }

    if let Some(ci) = std::env::var_os("CI") {
      if let Some(ci_value) = ci.to_str() {
        if matches!(
          ci_value,
          "TRAVIS"
            | "CIRCLECI"
            | "APPVEYOR"
            | "GITLAB_CI"
            | "GITHUB_ACTIONS"
            | "BUILDKITE"
            | "DRONE"
        ) {
          return ColorLevel::Ansi256;
        }
      } else {
        return ColorLevel::Ansi;
      }
    }

    if let Some(color_term) = std::env::var_os("COLORTERM") {
      if color_term == "truecolor" || color_term == "24bit" {
        return ColorLevel::TrueColor;
      }
    }

    if term.ends_with("-256color") || term.ends_with("256") {
      return ColorLevel::Ansi256;
    }

    ColorLevel::Ansi
  }
});

pub fn get_color_level() -> ColorLevel {
  *COLOR_LEVEL
}

/// Gets whether color should be used in the output.
///
/// This is determined by:
/// - The `FORCE_COLOR` environment variable (if set, enables color regardless of other settings)
/// - The `NO_COLOR` environment variable (if set and FORCE_COLOR is not set, disables color)
/// - If `set_use_color` has been called to explicitly set the color state
///
/// On Wasm targets, use `set_use_color(true)` to enable color output.
pub fn use_color() -> bool {
  USE_COLOR.load(std::sync::atomic::Ordering::Relaxed)
}

/// Whether the `FORCE_COLOR` environment variable is set
pub fn force_color() -> bool {
  *FORCE_COLOR
}

/// Sets whether color should be used in the output.
///
/// This overrides the default values set via the `FORCE_COLOR` and `NO_COLOR` env vars.
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

pub fn dimmed_gray<'a, S: fmt::Display + 'a>(s: S) -> Style<S> {
  let mut style_spec = ColorSpec::new();
  style_spec.set_dimmed(true).set_fg(Some(Ansi256(245)));
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
