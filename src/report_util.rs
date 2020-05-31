use std::fmt;
use std::io::Write;
use termcolor::Color::{Ansi256, Red};
use termcolor::{Ansi, ColorSpec, WriteColor};

#[cfg(windows)]
use termcolor::{BufferWriter, ColorChoice};

#[cfg(windows)]
pub fn enable_ansi() {
  BufferWriter::stdout(ColorChoice::AlwaysAnsi);
}

pub fn gray(s: String) -> impl fmt::Display {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Ansi256(8)));
  style(&s, style_spec)
}

pub fn red(s: String) -> impl fmt::Display {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Red));
  style(&s, style_spec)
}

pub fn cyan(s: String) -> impl fmt::Display {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Ansi256(14)));
  style(&s, style_spec)
}

fn style(s: &str, colorspec: ColorSpec) -> impl fmt::Display {
  let mut v = Vec::new();
  let mut ansi_writer = Ansi::new(&mut v);
  ansi_writer.set_color(&colorspec).unwrap();
  ansi_writer.write_all(s.as_bytes()).unwrap();
  ansi_writer.reset().unwrap();
  String::from_utf8_lossy(&v).into_owned()
}

pub fn report_filename(file_name: &String) {
  eprintln!("{}", cyan(String::from(format!("{} =>", file_name))));
}

pub fn report_line_src(line_index: &usize, line_src: &String) {
  eprintln!("  {}| {}", line_index, line_src);
}

pub fn place_glyph(line_index: &usize, col: &usize) {
  eprintln!(
    "  {}{}",
    " ".repeat(line_index.to_string().len() + col + 2),
    red("^".to_string())
  );
}

pub fn report_error(code: &String, message: &String) {
  eprintln!("  ({}) {}", gray(String::from(code)), message);
}
