use ratatui::{
  buffer::Buffer,
  layout::Rect,
  style::{Modifier, Style, Stylize},
  widgets::WidgetRef,
};
use tui_textarea::TextArea;

pub trait StylableWidget: Sized {
  fn style(&mut self, style: Style);

  #[allow(unused)]
  fn focus_style(&mut self, style: Option<Style>, focused: bool) {
    if let Some(style) = style {
      self.style(style);
    }
  }
}

pub trait EventfulState<E> {
  fn handle_ev(&mut self, event: E) -> Option<E>;
}

impl<'a, E: Into<tui_textarea::Input>> EventfulState<E> for TextArea<'a> {
  fn handle_ev(&mut self, event: E) -> Option<E> {
    self.input(event);
    None
  }
}

impl<'a> StylableWidget for TextArea<'a> {
  fn style(&mut self, style: Style) {
    self.set_style(style.patch(Into::<Style>::into(Modifier::UNDERLINED)));
  }

  fn focus_style(&mut self, style: Option<Style>, focused: bool) {
    if let Some(style) = style {
      self.set_style(style.add_modifier(Modifier::UNDERLINED));
    }

    if focused {
      self.set_cursor_style(self.cursor_line_style().reversed());
    } else {
      self.set_cursor_style(self.cursor_line_style());
    }
  }
}

pub trait WidgetRefMut {
  fn render_ref_mut(&mut self, area: Rect, buf: &mut Buffer);
}

impl<T: WidgetRef> WidgetRefMut for T {
  fn render_ref_mut(&mut self, area: Rect, buf: &mut Buffer) {
    self.render_ref(area, buf);
  }
}
