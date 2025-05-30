use std::fmt::Display;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
  layout::Rect,
  style::{Color, Style},
  widgets::StatefulWidget,
};

use super::{
  focus::{FocusManager, FocusableState},
  traits::EventfulState,
};

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

pub struct Dropdown<'a, T: Display> {
  style: Option<Style>,
  open_style: Option<Style>,
  selected_style: Option<Style>,
  state: &'a mut FocusableState<DropdownState<T>>,
}

impl<'a, T: Display> Dropdown<'a, T> {
  pub fn new(state: &'a mut FocusableState<DropdownState<T>>) -> Self {
    Dropdown {
      style: None,
      open_style: None,
      selected_style: None,
      state,
    }
  }

  pub fn line_width(&self) -> u16 {
    self
      .state
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

impl<'a, T: Display> StatefulWidget for Dropdown<'a, T> {
  type State = &'a FocusManager;

  fn render(
    self,
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    state: &mut Self::State,
  ) {
    let root_style = Style::reset().patch(
      state
        .focus
        .is_some_and(|f| self.state.focus_id == f)
        .then_some(self.state.focus_style)
        .flatten()
        .or(self.state.unfocus_style)
        .unwrap_or_default()
        .patch(self.style.unwrap_or_default()),
    );

    let focused = state.focus.is_some_and(|f| f == self.state.focus_id);
    self.state.open = self.state.open && focused;

    buf.set_string(
      area.x,
      area.y,
      if self.state.open { "▼ " } else { "▶ " },
      root_style,
    );

    let text_width = area.width.saturating_sub(2) as usize;

    buf.set_stringn(
      area.x + 2,
      area.y,
      format!("{}", self.state.options[self.state.choice]),
      text_width,
      root_style,
    );

    if self.state.open {
      let open_style = self.open_style.unwrap_or_else(|| Style {
        fg: Some(root_style.bg.unwrap_or(Color::Black)),
        bg: Some(root_style.fg.unwrap_or(Color::White)),
        ..root_style
      });
      let selected_style = self.selected_style.unwrap_or(root_style);

      for (i, option) in self.state.options.iter().enumerate() {
        buf.set_stringn(
          area.x + 2,
          area.y + i as u16 + 1,
          format!("{:<width$}", option, width = text_width),
          text_width,
          if i == self.state.choice {
            selected_style
          } else {
            open_style
          },
        );
      }
    }
  }
}
