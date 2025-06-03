use ratatui::{
  buffer::Buffer,
  crossterm::event::{KeyCode, KeyEvent},
  layout::Rect,
  text::Text,
  widgets::{Paragraph, WidgetRef},
};

use super::traits::EventfulState;

pub struct Scrollagraph<'a> {
  par: Paragraph<'a>,
  scroll_y: usize,
}

impl<'a> Scrollagraph<'a> {
  pub fn new<T: Into<Text<'a>>>(content: T) -> Self {
    Scrollagraph {
      par: Paragraph::new(content.into()),
      scroll_y: 0,
    }
  }
}

impl<'a> WidgetRef for Scrollagraph<'a> {
  fn render_ref(&self, area: Rect, buf: &mut Buffer) {
    self.par.render_ref(area, buf);
  }
}

impl<'a> EventfulState<KeyEvent> for Scrollagraph<'a> {
  fn handle_ev(&mut self, event: KeyEvent) -> Option<KeyEvent> {
    match event {
      key_event!(KeyCode::Up) => {
        self.scroll_y = self.scroll_y.saturating_sub(1);
      }
      key_event!(KeyCode::Down) => {
        self.scroll_y = self.scroll_y.saturating_add(1);
      }
      key_event!(KeyCode::PageUp) => {
        self.scroll_y = self.scroll_y.saturating_sub(20);
      }
      key_event!(KeyCode::PageDown) => {
        self.scroll_y = self.scroll_y.saturating_add(20);
      }
      ev => return Some(ev),
    }

    self.par = std::mem::take(&mut self.par).scroll((self.scroll_y as u16, 0));

    None
  }
}
