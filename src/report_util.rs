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

pub fn report_location(file_name: &str, line_index: usize, col: usize) {
  let location = if file_name.contains('/')
    || file_name.contains('\\')
    || file_name.starts_with("./")
  {
    file_name.to_string()
  } else {
    format!("./{}", file_name)
  };

  eprintln!(
    "{}",
    cyan(format!(" --> {}:{}:{}", location, line_index, col))
  );
}

pub fn report_line_src(line_index: usize, line_src: &str) {
  eprintln!("{}|", " ".repeat(line_index.to_string().len() + 1),);
  eprintln!("{} | {}", line_index, line_src);
}

pub fn place_glyphes(line_index: usize, col: usize, length: usize) {
  eprintln!(
    "{}|{}{}",
    " ".repeat(line_index.to_string().len() + 1),
    " ".repeat(col + 3),
    red("^".repeat(length))
  );
  eprintln!("{}|", " ".repeat(line_index.to_string().len() + 1),);
}

pub fn report_error(code: &str, message: &str) {
  eprintln!("({}) {}", gray(code.to_string()), message);
}
