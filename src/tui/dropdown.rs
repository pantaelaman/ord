use std::{fmt::Display, marker::PhantomData};

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
  layout::Rect,
  style::{Color, Style},
  widgets::StatefulWidget,
};

use super::traits::{EventfulState, StylableWidget};

pub struct DropdownState<T: Display> {
  options: Vec<T>,
  choice: usize,
  open: bool,
  changed: bool,
}

impl<T: Display> EventfulState<KeyEvent> for DropdownState<T> {
  fn handle_ev(&mut self, event: KeyEvent) -> Option<KeyEvent> {
    match event {
      key_event!(KeyCode::Enter) => self.open = !self.open,
      event => {
        if self.open {
          match event {
            key_event!(KeyCode::Up) => self.prev(),
            key_event!(KeyCode::Down) => self.next(),
            ev => return Some(ev),
          }
        }
      }
    }
    None
  }
}

impl<T: Display> DropdownState<T> {
  pub fn new<I: IntoIterator<Item = T>>(options: I) -> Self {
    DropdownState {
      options: options.into_iter().collect(),
      choice: 0,
      open: false,
      changed: true,
    }
  }

  pub fn line_width(&self) -> u16 {
    self
      .options
      .iter()
      .map(|o| format!("{}", o).len() as u16)
      .max()
      .unwrap_or_default()
      + 2
  }

  pub fn limit_area_width(&self, area: Rect) -> Rect {
    Rect {
      width: std::cmp::min(area.width, self.line_width()),
      ..area
    }
  }

  pub fn next(&mut self) {
    self.choice = (self.choice + 1) % self.options.len();
    self.changed = true;
  }

  pub fn prev(&mut self) {
    let num_options = self.options.len();
    self.choice = (self.choice + num_options - 1) % num_options;
    self.changed = true;
  }

  pub fn changed(&mut self) -> bool {
    let res = self.changed;
    self.changed = false;
    res
  }

  pub fn get(&self) -> &T {
    &self.options[self.choice]
  }
}

impl<T: Display + PartialEq> DropdownState<T> {
  pub fn with_initial<F>(self, cond: F) -> Self
  where
    F: Fn(&T) -> bool,
  {
    Self {
      choice: self.options.iter().position(cond).unwrap(),
      ..self
    }
  }
}

pub struct Dropdown<T> {
  style: Option<Style>,
  open_style: Option<Style>,
  selected_style: Option<Style>,
  focused: bool,
  _phantom: PhantomData<T>,
}

impl<T> Default for Dropdown<T> {
  fn default() -> Self {
    Dropdown {
      style: None,
      open_style: None,
      selected_style: None,
      focused: true,
      _phantom: PhantomData,
    }
  }
}

#[allow(unused)]
impl<T> Dropdown<T> {
  pub fn style(self, style: Style) -> Self {
    Dropdown {
      style: Some(style),
      ..self
    }
  }

  pub fn open_style(self, style: Style) -> Self {
    Dropdown {
      open_style: Some(style),
      ..self
    }
  }

  pub fn selected_style(self, style: Style) -> Self {
    Dropdown {
      selected_style: Some(style),
      ..self
    }
  }
}

impl<T> StylableWidget for Dropdown<T> {
  fn style(&mut self, style: Style) {
    self.style = Some(self.style.map(|s| s.patch(style)).unwrap_or(style));
  }

  fn focus_style(&mut self, style: Option<Style>, focused: bool) {
    self.focused = focused;
    if let Some(style) = style {
      self.style(style);
    }
  }
}

impl<T: Display> StatefulWidget for Dropdown<T> {
  type State = DropdownState<T>;

  fn render(
    self,
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut Self::State,
  ) {
    let focused = self.focused;
    state.open = state.open && focused;

    let root_style = self.style.unwrap_or_default();

    buf.set_string(
      area.x,
      area.y,
      if state.open { "▼ " } else { "▶ " },
      root_style,
    );

    let text_width = area.width.saturating_sub(2) as usize;

    buf.set_stringn(
      area.x + 2,
      area.y,
      format!("{}", state.options[state.choice]),
      text_width,
      root_style,
    );

    if state.open {
      let open_style = self.open_style.unwrap_or_else(|| Style {
        fg: Some(root_style.bg.unwrap_or(Color::Black)),
        bg: Some(root_style.fg.unwrap_or(Color::White)),
        ..root_style
      });
      let selected_style = self.selected_style.unwrap_or(root_style);

      for (i, option) in state.options.iter().enumerate() {
        buf.set_stringn(
          area.x + 2,
          area.y + i as u16 + 1,
          format!("{:<text_width$}", option),
          text_width,
          if i == state.choice {
            selected_style
          } else {
            open_style
          },
        );
      }
    }
  }
}
