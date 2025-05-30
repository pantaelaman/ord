use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
  layout::Rect,
  style::{Color, Modifier, Style},
  widgets::{Block, StatefulWidget, Widget},
};

use super::traits::{EventfulState, StylableWidget};

pub struct LineInput {
  style: Style,
  show_cursor: bool,
  strict_width: Option<u16>,
}

impl Default for LineInput {
  fn default() -> Self {
    LineInput {
      style: Modifier::UNDERLINED.into(),
      show_cursor: true,
      strict_width: None,
    }
  }
}

impl LineInput {
  pub fn strict_width(self, width: u16) -> Self {
    LineInput {
      strict_width: Some(width),
      ..self
    }
  }
}

impl StylableWidget for LineInput {
  fn style(self, style: Style) -> Self {
    LineInput { style, ..self }
  }

  fn focus_style(self, style: Option<Style>, focused: bool) -> Self {
    LineInput {
      style: self.style.patch(style.unwrap_or_default()),
      show_cursor: focused,
      ..self
    }
  }
}

#[derive(Debug, Default)]
pub struct LineInputState {
  content: Vec<char>,
  cursor: usize,
}

impl LineInputState {
  pub fn clear(&mut self) {
    self.content.clear();
    self.cursor = 0;
  }

  pub fn get_str(&self) -> &str {
    self.content.as_slice()
  }
}
impl EventfulState<KeyEvent> for LineInputState {
  fn handle_ev(&mut self, ev: KeyEvent) -> Option<KeyEvent> {
    match ev {
      key_event!(KeyCode::Char(c), KeyModifiers::NONE, KeyModifiers::SHIFT) => {
        self.content.insert(self.cursor, c);
        self.cursor += 1;
      }
      key_event!(KeyCode::Backspace) => {
        if self.cursor > 0 {
          self.content.remove(self.cursor - 1);
          self.cursor -= 1;
        }
      }
      key_event!(KeyCode::Left) => {
        self.cursor = self.cursor.saturating_sub(1);
      }
      key_event!(KeyCode::Right) => {
        self.cursor = self.cursor.saturating_add(1);
      }
      key_event!(KeyCode::Home) => {
        self.cursor = 0;
      }
      key_event!(KeyCode::End) => {
        self.cursor = self.content.len();
      }
      ev => return Some(ev),
    }

    None
  }
}

impl StatefulWidget for LineInput {
  type State = LineInputState;

  fn render(
    self,
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut Self::State,
  ) {
    let max_width = self
      .strict_width
      .map(|v| v as usize)
      .unwrap_or_else(|| state.content.len());

    let offset_x = std::cmp::min(
      state.content.len().saturating_sub(max_width),
      state.cursor,
    );
    let slice = &state.content[offset_x..];
    let actual_cursor = state.cursor - offset_x;

    buf.set_stringn(
      area.x,
      area.y,
      format!("{:<width$}", slice, width = max_width),
      std::cmp::min(max_width, area.width as usize),
      self.style,
    );
    if self.show_cursor {
      buf.set_style(
        Rect::new(area.x + actual_cursor as u16, area.y, 1, 1),
        Style {
          fg: self.style.bg.or(Some(Color::Black)),
          bg: self.style.fg.or(Some(Color::White)),
          ..self.style
        },
      );
    }
  }
}
