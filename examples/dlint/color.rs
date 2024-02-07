// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use deno_ast::{
  lex,
  swc::parser::token::{Token, Word},
  MediaType, TokenOrComment,
};
use if_chain::if_chain;
use pulldown_cmark::{Options, Parser, Tag};

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
      ListKind::Unordered => "  - ".to_string(),
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
    Paragraph | Heading(_, _, _) | BlockQuote | CodeBlock(_) => 2,
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

  fn run(mut self, parser: Parser<'_, '_>) -> String {
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
    use pulldown_cmark::{CodeBlockKind, HeadingLevel, Tag::*};
    match tag {
      Paragraph => vec![],
      Heading(HeadingLevel::H1, _, _) => {
        vec![
          Attribute::Italic,
          Attribute::Underline,
          Attribute::Bold,
          Attribute::Magenta,
        ]
      }
      Heading(HeadingLevel::H2, _, _) => {
        vec![Attribute::Bold, Attribute::Magenta]
      }
      Heading(HeadingLevel::H3, _, _) => vec![Attribute::Magenta],
      Heading(_, _, _) => vec![Attribute::Bold],
      BlockQuote => vec![Attribute::Gray],
      CodeBlock(kind) => {
        if is_start {
          if_chain! {
            if let CodeBlockKind::Fenced(info) = kind;
            if let Some(media_type) = try_code_block_to_media_type(info);
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
          use std::fmt::Write;
          write!(self.buffer, "({})", url).unwrap();
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
        let range = item.range;

        out_line.replace_range(
          range.start + offset..range.end + offset,
          &match item.inner {
            TokenOrComment::Token(token) => match token {
              Token::Str { .. } | Token::Template { .. } | Token::BackQuote => {
                decorate(&line[range], Attribute::Green)
              }
              Token::Regex(_, _) => decorate(&line[range], Attribute::Red),
              Token::Num { .. } | Token::BigInt { .. } => {
                decorate(&line[range], Attribute::Yellow)
              }
              Token::Word(word) => match word {
                Word::True | Word::False | Word::Null => {
                  decorate(&line[range], Attribute::Yellow)
                }
                Word::Keyword(_) => decorate(&line[range], Attribute::Cyan),
                Word::Ident(ident) => match ident.as_ref() {
                  "undefined" => decorate(&line[range], Attribute::Gray),
                  "Infinity" | "NaN" => {
                    decorate(&line[range], Attribute::Yellow)
                  }
                  "async" | "of" | "enum" | "type" | "interface" => {
                    decorate(&line[range], Attribute::Cyan)
                  }
                  _ => line[range].to_string(),
                },
              },
              _ => line[range].to_string(),
            },
            TokenOrComment::Comment { .. } => {
              decorate(&line[range], Attribute::Gray)
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

fn try_code_block_to_media_type(value: &str) -> Option<MediaType> {
  match value {
    "javascript" => Some(MediaType::JavaScript),
    "typescript" => Some(MediaType::TypeScript),
    "jsx" => Some(MediaType::Jsx),
    "tsx" => Some(MediaType::Tsx),
    "dts" => Some(MediaType::Dts),
    _ => None,
  }
}
