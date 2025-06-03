use ratatui::{
  buffer::Buffer,
  layout::Rect,
  style::Style,
  widgets::{StatefulWidget, Widget},
};

use super::traits::StylableWidget;

pub struct LeftLabelled<W> {
  label: String,
  style: Option<Style>,
  width: Option<u16>,
  child: W,
}

impl<W> LeftLabelled<W> {
  pub fn new<L: Into<String>>(label: L, child: W) -> Self {
    Self {
      label: label.into(),
      child,
      style: None,
      width: None,
    }
  }

  pub fn width(self, width: u16) -> Self {
    Self {
      width: Some(width),
      ..self
    }
  }

  fn render_label(&self, area: Rect, buf: &mut Buffer) -> Rect {
    buf.set_string(area.x, area.y, &self.label, self.style.unwrap_or_default());
    let offset =
      std::cmp::max(self.label.len() as u16 + 1, self.width.unwrap_or(0));
    Rect {
      x: area.x + offset,
      width: area.width - offset,
      ..area
    }
  }
}

impl<W: StylableWidget> StylableWidget for LeftLabelled<W> {
  fn style(&mut self, style: Style) {
    self.child.style(style);
  }

  fn focus_style(&mut self, style: Option<Style>, focused: bool) {
    self.child.focus_style(style, focused);
  }
}

impl<W: Widget> Widget for LeftLabelled<W> {
  fn render(
    self,
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
  ) where
    Self: Sized,
  {
    let area = self.render_label(area, buf);
    self.child.render(area, buf);
  }
}

impl<W: StatefulWidget> StatefulWidget for LeftLabelled<W> {
  type State = <W as StatefulWidget>::State;

  fn render(
    self,
    area: Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut Self::State,
  ) {
    let area = self.render_label(area, buf);
    self.child.render(area, buf, state);
  }
}
