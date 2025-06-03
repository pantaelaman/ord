use ratatui::{
  crossterm::event::{KeyCode, KeyEvent},
  style::{Color, Modifier, Style},
  widgets::Widget,
};
use tui_textarea::TextArea;

use super::traits::{EventfulState, StylableWidget};

pub struct LineInput<'a> {
  pub text_area: TextArea<'a>,
}

impl<'a> Into<TextArea<'a>> for LineInput<'a> {
  fn into(self) -> TextArea<'a> {
    self.text_area
  }
}

impl<'a> LineInput<'a> {
  pub fn default_area() -> Self {
    let mut text_area = TextArea::default();
    text_area.set_cursor_line_style(Style::default());
    Self { text_area }
  }

  pub fn default_or_with(text: Option<String>) -> Self {
    match text {
      Some(text) => Self::default_with(text),
      None => Self::default(),
    }
  }

  pub fn default_with(text: String) -> Self {
    let mut text_area = TextArea::new(vec![text]);
    text_area.set_style(Modifier::UNDERLINED.into());
    text_area.set_cursor_line_style(Style::default());
    text_area.set_placeholder_style(
      Style::new()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::UNDERLINED),
    );
    Self { text_area }
  }
}

impl<'a> Default for LineInput<'a> {
  fn default() -> Self {
    let mut text_area = TextArea::default();
    text_area.set_style(Modifier::UNDERLINED.into());
    text_area.set_cursor_line_style(Style::default());
    text_area.set_placeholder_style(
      Style::new()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::UNDERLINED),
    );
    LineInput { text_area }
  }
}

impl<'a> std::ops::Deref for LineInput<'a> {
  type Target = TextArea<'a>;

  fn deref(&self) -> &Self::Target {
    &self.text_area
  }
}

impl<'a> std::ops::DerefMut for LineInput<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.text_area
  }
}

impl<'a> EventfulState<KeyEvent> for LineInput<'a> {
  fn handle_ev(&mut self, event: KeyEvent) -> Option<KeyEvent> {
    match event {
      key_event!(KeyCode::Enter) => Some(event),
      ev => self.text_area.handle_ev(ev),
    }
  }
}

impl<'a> StylableWidget for LineInput<'a> {
  fn style(&mut self, style: Style) {
    StylableWidget::style(&mut self.text_area, style);
  }

  fn focus_style(&mut self, style: Option<Style>, focused: bool) {
    StylableWidget::focus_style(&mut self.text_area, style, focused);
  }
}

impl<'a> Widget for &LineInput<'a> {
  fn render(
    self,
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
  ) where
    Self: Sized,
  {
    self.text_area.render(area, buf);
  }
}
