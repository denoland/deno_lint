// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::lexer::{lex, MediaType, TokenOrComment};
use if_chain::if_chain;
use pulldown_cmark::{Options, Parser, Tag};
use std::convert::TryFrom;
use swc_ecmascript::parser::token::{Token, Word};

pub fn colorize_markdown(input: &str) -> String {
  let mut options = Options::empty();
  options.insert(Options::ENABLE_STRIKETHROUGH);
  let parser = Parser::new_ext(input, options);
  let colorizer = MarkdownColorizer::new();
  colorizer.run(parser)
}

const RESET_CODE: &str = "\u{001b}[0m";

#[derive(Debug, Clone, Copy)]
enum CodeBlockLang {
  Known(MediaType),
  Unknown,
}

#[derive(Debug, Clone, Copy)]
enum ListKind {
  Ordered { current_number: u64 },
  Unordered,
}

impl ListKind {
  fn render(&mut self) -> String {
    match self {
      ListKind::Ordered { current_number } => {
        let ret = format!("  {}. ", *current_number);
        *current_number += 1;
        ret
      }
      ListKind::Unordered => {
        format!("  - ")
      }
    }
  }
}

struct MarkdownColorizer {
  attr_stack: Vec<Attribute>,
  code_block: Option<CodeBlockLang>,
  list: Option<ListKind>,
  buffer: String,
}

fn trailing_newlines(tag: &Tag) -> String {
  use Tag::*;
  let num_newlines = match tag {
    Paragraph | Heading(_) | BlockQuote | CodeBlock(_) => 2,
    List(_)
    | Item
    | FootnoteDefinition(_)
    | Table(_)
    | TableHead
    | TableRow
    | TableCell => 1,
    Emphasis | Strong | Strikethrough | Link(_, _, _) | Image(_, _, _) => 0,
  };
  "\n".repeat(num_newlines)
}

impl MarkdownColorizer {
  fn new() -> MarkdownColorizer {
    Self {
      attr_stack: vec![],
      code_block: None,
      list: None,
      buffer: String::new(),
    }
  }

  fn run<'input>(mut self, parser: Parser<'input>) -> String {
    for event in parser {
      use pulldown_cmark::Event::*;
      match event {
        Start(tag) => {
          let attrs = self.handle_tag(&tag, true);
          for attr in &attrs {
            self.buffer.push_str(attr.as_ansi_code());
          }
          self.attr_stack.extend(attrs);
        }
        End(tag) => {
          self.buffer.push_str(RESET_CODE);
          let attrs = self.handle_tag(&tag, false);
          self
            .attr_stack
            .truncate(self.attr_stack.len() - attrs.len());
          for attr in &self.attr_stack {
            self.buffer.push_str(attr.as_ansi_code());
          }
          self.buffer.push_str(&trailing_newlines(&tag));
        }
        Text(text) => {
          if let Some(lang) = self.code_block {
            self.buffer.push_str(&colorize_code_block(lang, &text));
          } else {
            self.buffer.push_str(&text);
          }
        }
        Html(html) => {
          self.buffer.push_str(&html);
        }
        Code(code) => {
          let attr = Attribute::Green;
          self.buffer.push_str(attr.as_ansi_code());
          self.buffer.push_str(&code);
          self.buffer.push_str(RESET_CODE);
          for attr in &self.attr_stack {
            self.buffer.push_str(attr.as_ansi_code());
          }
        }
        FootnoteReference(_) => {}
        SoftBreak | HardBreak => {
          self.buffer.push('\n');
        }
        Rule => {
          self.buffer.push_str(&"-".repeat(80));
        }
        TaskListMarker(checked) => {
          if checked {
            self.buffer.push_str("[x]");
          } else {
            self.buffer.push_str("[ ]");
          }
        }
      }
    }

    self.buffer.trim_end_matches('\n').to_string()
  }

  fn handle_tag(&mut self, tag: &Tag, is_start: bool) -> Vec<Attribute> {
    use pulldown_cmark::{CodeBlockKind, Tag::*};
    match tag {
      Paragraph => vec![],
      Heading(1) => {
        vec![
          Attribute::Italic,
          Attribute::Underline,
          Attribute::Bold,
          Attribute::Magenta,
        ]
      }
      Heading(2) => vec![Attribute::Bold, Attribute::Magenta],
      Heading(3) => vec![Attribute::Magenta],
      Heading(_) => vec![Attribute::Bold],
      BlockQuote => vec![Attribute::Gray],
      CodeBlock(kind) => {
        if is_start {
          if_chain! {
            if let CodeBlockKind::Fenced(info) = kind;
            if let Ok(media_type) = MediaType::try_from(&**info);
            then {
              self.code_block = Some(CodeBlockLang::Known(media_type))
            } else {
              self.code_block = Some(CodeBlockLang::Unknown)
            }
          }
        } else {
          self.code_block = None
        };
        vec![]
      }
      List(Some(n)) => {
        self.list = if is_start {
          Some(ListKind::Ordered { current_number: *n })
        } else {
          None
        };
        vec![]
      }
      List(None) => {
        self.list = if is_start {
          Some(ListKind::Unordered)
        } else {
          None
        };
        vec![]
      }
      Item => {
        if is_start {
          let list_kind =
            self.list.as_mut().expect("ListKind should be set, but not");
          self.buffer.push_str(&list_kind.render());
        }
        vec![]
      }
      // TODO(magurotuna) we should implement this
      FootnoteDefinition(_) => vec![],
      // TODO(magurotuna) we should implement this
      Table(_) | TableHead | TableRow | TableCell => vec![],
      Emphasis => vec![Attribute::Italic],
      Strong => vec![Attribute::Bold],
      Strikethrough => vec![Attribute::Strikethrough],
      Link(_link_type, url, _title) => {
        if !is_start {
          self.buffer.push_str(&format!("({url})", url = url));
        }
        vec![]
      }
      // TODO(magurotuna) we should implement this
      Image(_, _, _) => vec![],
    }
  }
}

#[derive(Debug, Clone, Copy)]
enum Attribute {
  Bold,
  Italic,
  Underline,
  #[allow(dead_code)]
  Reversed,
  Strikethrough,
  #[allow(dead_code)]
  Black,
  Red,
  Green,
  Yellow,
  #[allow(dead_code)]
  Blue,
  Magenta,
  Cyan,
  #[allow(dead_code)]
  White,
  Gray,
}

impl Attribute {
  fn as_ansi_code(&self) -> &'static str {
    use Attribute::*;
    match *self {
      Bold => "\u{001b}[1m",
      Italic => "\u{001b}[3m",
      Underline => "\u{001b}[4m",
      Reversed => "\u{001b}[7m",
      Strikethrough => "\u{001b}[9m",
      Black => "\u{001b}[30m",
      Red => "\u{001b}[31m",
      Green => "\u{001b}[32m",
      Yellow => "\u{001b}[33m",
      Blue => "\u{001b}[34m",
      Magenta => "\u{001b}[35m",
      Cyan => "\u{001b}[36m",
      White => "\u{001b}[37m",
      Gray => "\u{001b}[38;5;245m",
    }
  }
}

fn colorize_code_block(lang: CodeBlockLang, src: &str) -> String {
  fn decorate(s: &str, attr: Attribute) -> String {
    format!("{}{}{}", attr.as_ansi_code(), s, RESET_CODE)
  }

  if let CodeBlockLang::Known(media_type) = lang {
    let mut v = Vec::new();

    for line in src.split('\n') {
      // Ref: https://github.com/denoland/deno/blob/a0c0daac24c496e49e7c0abaae12f34723785a7d/cli/tools/repl.rs#L251-L298
      let mut out_line = String::from(line);
      for item in lex(line, media_type) {
        let offset = out_line.len() - line.len();
        let span = item.span_as_range();

        out_line.replace_range(
          span.start + offset..span.end + offset,
          &match item.inner {
            TokenOrComment::Token(token) => match token {
              Token::Str { .. } | Token::Template { .. } | Token::BackQuote => {
                decorate(&line[span], Attribute::Green)
              }
              Token::Regex(_, _) => decorate(&line[span], Attribute::Red),
              Token::Num(_) | Token::BigInt(_) => {
                decorate(&line[span], Attribute::Yellow)
              }
              Token::Word(word) => match word {
                Word::True | Word::False | Word::Null => {
                  decorate(&line[span], Attribute::Yellow)
                }
                Word::Keyword(_) => decorate(&line[span], Attribute::Cyan),
                Word::Ident(ident) => {
                  if ident == *"undefined" {
                    decorate(&line[span], Attribute::Gray)
                  } else if ident == *"Infinity" || ident == *"NaN" {
                    decorate(&line[span], Attribute::Yellow)
                  } else if matches!(
                    ident.as_ref(),
                    "async" | "of" | "enum" | "type" | "interface"
                  ) {
                    decorate(&line[span], Attribute::Cyan)
                  } else {
                    line[span].to_string()
                  }
                }
              },
              _ => line[span].to_string(),
            },
            TokenOrComment::Comment { .. } => {
              decorate(&line[span], Attribute::Gray)
            }
          },
        );
      }
      v.push(out_line.indent(4));
    }

    v.join("\n")
  } else {
    src.split('\n').map(|line| line.indent(4)).join_by("\n")
  }
}

trait Indent {
  fn indent(self, width: usize) -> String;
}

impl Indent for String {
  fn indent(self, width: usize) -> String {
    format!("{}{}", " ".repeat(width), self)
  }
}

impl Indent for &str {
  fn indent(self, width: usize) -> String {
    format!("{}{}", " ".repeat(width), self)
  }
}

trait JoinBy {
  fn join_by(self, sep: &str) -> String;
}

impl<I> JoinBy for I
where
  I: Iterator<Item = String>,
{
  fn join_by(self, sep: &str) -> String {
    self.collect::<Vec<_>>().join(sep)
  }
}
