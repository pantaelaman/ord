use crossterm::event::KeyEvent;
use ratatui::style::Style;

pub trait StylableWidget: Sized {
  fn style(self, style: Style) -> Self;

  fn focus_style(self, style: Option<Style>, focused: bool) -> Self {
    if let Some(style) = style {
      self.style(style)
    } else {
      self
    }
  }
}

pub trait EventfulState<E> {
  fn handle_ev(&mut self, event: E) -> Option<E>;
}

pub trait HandleableEvent: Sized {
  fn handle_with<W: EventfulState<Self>>(self, state: &mut W) -> Option<Self>;
}

impl HandleableEvent for KeyEvent {
  fn handle_with<W: EventfulState<Self>>(self, state: &mut W) -> Option<Self> {
    state.handle_ev(self)
  }
}

pub trait OptionEventExt<E: HandleableEvent>: Sized {
  fn chain_with<W: EventfulState<E>>(self, state: &mut W) -> Option<E>;
}

impl<E: HandleableEvent> OptionEventExt<E> for Option<E> {
  fn chain_with<W: EventfulState<E>>(self, state: &mut W) -> Option<E> {
    self.and_then(|e| e.handle_with(state))
  }
}
