// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::lexer::{lex, MediaType, TokenOrComment};
use std::convert::TryFrom;
use swc_ecmascript::parser::token::{Token, Word};

pub trait Colorize {
  fn colorize(self) -> String;
}

impl Colorize for markdown::Block {
  fn colorize(self) -> String {
    use markdown::Block::*;
    match self {
      Header(spans, 1) => {
        let style = ansi_term::Style::new()
          .bold()
          .underline()
          .italic()
          .fg(ansi_term::Color::Purple);
        style.paint(spans.colorize()).to_string().linebreak()
      }
      Header(spans, 2 | 3) => {
        let style = ansi_term::Style::new()
          .bold()
          .underline()
          .fg(ansi_term::Color::Purple);
        style.paint(spans.colorize()).to_string().linebreak()
      }
      Header(spans, 4) => {
        let style = ansi_term::Style::new().bold().fg(ansi_term::Color::Purple);
        style.paint(spans.colorize()).to_string().linebreak()
      }
      Header(spans, _level) => {
        let style = ansi_term::Style::new().fg(ansi_term::Color::Purple);
        style.paint(spans.colorize()).to_string().linebreak()
      }
      Paragraph(spans) => spans.colorize().linebreak(),
      Blockquote(blocks) => {
        let style = ansi_term::Style::new().dimmed();
        style.paint(blocks.colorize()).to_string().linebreak()
      }
      CodeBlock(Some(info), content) => {
        if let Ok(media_type) = MediaType::try_from(info.as_str()) {
          let mut v = Vec::new();

          for line in content.split('\n') {
            // Ref: https://github.com/denoland/deno/blob/a0c0daac24c496e49e7c0abaae12f34723785a7d/cli/tools/repl.rs#L251-L298
            let mut out_line = String::from(line);
            for item in lex(line, media_type) {
              let offset = out_line.len() - line.len();
              let span = item.span_as_range();

              out_line.replace_range(
                span.start + offset..span.end + offset,
                &match item.inner {
                  TokenOrComment::Token(token) => match token {
                    Token::Str { .. }
                    | Token::Template { .. }
                    | Token::BackQuote => {
                      ansi_term::Color::Green.paint(&line[span]).to_string()
                    }
                    Token::Regex(_, _) => {
                      ansi_term::Color::Red.paint(&line[span]).to_string()
                    }
                    Token::Num(_) | Token::BigInt(_) => {
                      ansi_term::Color::Yellow.paint(&line[span]).to_string()
                    }
                    Token::Word(word) => match word {
                      Word::True | Word::False | Word::Null => {
                        ansi_term::Color::Yellow.paint(&line[span]).to_string()
                      }
                      Word::Keyword(_) => {
                        ansi_term::Color::Cyan.paint(&line[span]).to_string()
                      }
                      Word::Ident(ident) => {
                        if ident == *"undefined" {
                          ansi_term::Color::Fixed(8)
                            .paint(&line[span])
                            .to_string()
                        } else if ident == *"Infinity" || ident == *"NaN" {
                          ansi_term::Color::Yellow
                            .paint(&line[span])
                            .to_string()
                        } else if matches!(
                          ident.as_ref(),
                          "async" | "of" | "enum" | "type" | "interface"
                        ) {
                          ansi_term::Color::Cyan.paint(&line[span]).to_string()
                        } else {
                          line[span].to_string()
                        }
                      }
                    },
                    _ => line[span].to_string(),
                  },
                  TokenOrComment::Comment { .. } => {
                    ansi_term::Color::Fixed(8).paint(&line[span]).to_string()
                  }
                },
              );
            }
            v.push(out_line.indent(4));
          }

          v.join("\n").linebreak()
        } else {
          content
            .split('\n')
            .map(|line| line.indent(4))
            .join_by("\n")
            .linebreak()
        }
      }
      CodeBlock(None, content) => content
        .split('\n')
        .map(|line| line.indent(4))
        .join_by("\n")
        .linebreak(),
      OrderedList(list_items, _list_type) => list_items
        .into_iter()
        .enumerate()
        .map(|(idx, li)| format!("{}. {}", idx, li.colorize()).indent(2))
        .join_by("\n")
        .linebreak(),
      UnorderedList(list_items) => list_items
        .into_iter()
        .map(|li| format!("• {}", li.colorize()).indent(2))
        .join_by("\n")
        .linebreak(),
      Raw(content) => content.linebreak(),
      Hr => ansi_term::Color::Fixed(8)
        .paint("-".repeat(80))
        .to_string()
        .linebreak(),
    }
  }
}

impl Colorize for Vec<markdown::Block> {
  fn colorize(self) -> String {
    self.into_iter().map(Colorize::colorize).join_by("\n")
  }
}

impl Colorize for markdown::Span {
  fn colorize(self) -> String {
    use markdown::Span::*;
    match self {
      Break => "\n".to_string(),
      Text(text) => text,
      Code(code) => ansi_term::Color::Green.paint(code).to_string(),
      Link(label, url, _title) => {
        format!("[{label}]({url})", label = label, url = url)
      }
      Image(alt, url, _title) => {
        format!("![{alt}]({url})", alt = alt, url = url)
      }
      Emphasis(spans) => {
        let style = ansi_term::Style::new().italic();
        style.paint(spans.colorize()).to_string()
      }
      Strong(spans) => {
        let style = ansi_term::Style::new().bold();
        style.paint(spans.colorize()).to_string()
      }
    }
  }
}

impl Colorize for Vec<markdown::Span> {
  fn colorize(self) -> String {
    self.into_iter().map(Colorize::colorize).join_by("")
  }
}

impl Colorize for markdown::ListItem {
  fn colorize(self) -> String {
    use markdown::ListItem::*;
    match self {
      Simple(spans) => spans.colorize(),
      Paragraph(blocks) => blocks.colorize(),
    }
  }
}

trait Linebreak {
  fn linebreak(self) -> String;
}

impl Linebreak for String {
  fn linebreak(self) -> String {
    format!("{}\n", self)
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
