pub trait Colorize {
  fn colorize(self) -> String;
}

impl Colorize for markdown::Block {
  fn colorize(self) -> String {
    use markdown::Block::*;
    match self {
      Header(spans, level) if level == 1 => {
        let style = ansi_term::Style::new()
          .bold()
          .underline()
          .italic()
          .fg(ansi_term::Color::Purple);
        style.paint(spans.colorize()).to_string().linebreak()
      }
      Header(spans, _level) => {
        let style = ansi_term::Style::new()
          .bold()
          .underline()
          .fg(ansi_term::Color::Purple);
        style.paint(spans.colorize()).to_string().linebreak()
      }
      Paragraph(spans) => spans.colorize().linebreak(),
      Blockquote(blocks) => {
        let style = ansi_term::Style::new().dimmed();
        style.paint(blocks.colorize()).to_string().linebreak()
      }
      CodeBlock(info, content)
        if matches!(
          info.as_deref(),
          Some("javascript" | "typescript" | "js" | "ts" | "jsx" | "tsx")
        ) =>
      {
        // TODO(magurotuna) syntax highlight
        content
          .split('\n')
          .map(|line| format!("{}", line).indent(4))
          .join_by("\n")
          .linebreak()
      }
      CodeBlock(_info, content) => {
        content.split('\n').map(|line| line.indent(4)).join_by("\n")
      }
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
      Raw(content) => content,
      Hr => "-".repeat(80),
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
      Code(code) => ansi_term::Color::Green.paint(code).to_string(), // todo
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
