use ratatui::{
  layout::Rect,
  style::Style,
  widgets::{StatefulWidget, Widget},
};

use super::traits::StylableWidget;

pub struct LeftLabelled<W> {
  label: String,
  style: Option<Style>,
  child: W,
}

impl<W> LeftLabelled<W> {
  pub fn new<L: Into<String>>(label: L, child: W) -> Self {
    Self {
      label: label.into(),
      child,
      style: None,
    }
  }
}

impl<W: StylableWidget> StylableWidget for LeftLabelled<W> {
  fn style(self, style: Style) -> Self {
    Self {
      child: self.child.style(style),
      ..self
    }
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
    buf.set_string(area.x, area.y, &self.label, self.style.unwrap_or_default());
    let offset = self.label.len() as u16 + 1;
    self.child.render(
      Rect {
        x: area.x + offset,
        width: area.width - offset,
        ..area
      },
      buf,
    );
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
    buf.set_string(area.x, area.y, &self.label, self.style.unwrap_or_default());
    let offset = self.label.len() as u16 + 1;
    self.child.render(
      Rect {
        x: area.x + offset,
        width: area.width - offset,
        ..area
      },
      buf,
      state,
    );
  }
}
