use ratatui::style::Style;

use super::traits::{EventfulState, StylableWidget};

pub struct FocusManager<T> {
  focus_style: Option<Style>,
  unfocus_style: Option<Style>,
  chunks: Vec<FocusChunk<T>>,
  active: Option<usize>,
}

impl<T> Default for FocusManager<T> {
  fn default() -> Self {
    Self {
      focus_style: None,
      unfocus_style: None,
      chunks: Vec::new(),
      active: None,
    }
  }
}

#[allow(unused)]
impl<'t, T: 't + PartialEq> FocusManager<T> {
  fn is_focused(&self, tag: &T) -> bool {
    self.active.is_some_and(|idx| tag == &self.chunks[idx].tag)
  }

  pub fn new_from_existing(&mut self, tag: T) -> FocusableBuilder<T> {
    self.enable(&tag);
    FocusableBuilder { tag }
  }

  pub fn enable(&mut self, tag: &T) {
    for chunk in self.chunks.iter_mut() {
      if chunk.tag == *tag {
        chunk.enabled = true;
      }
    }

    if let Some(focus) = self.active {
      self.set_focus(focus);
    }
  }

  pub fn disable(&mut self, tag: &T) {
    for chunk in self.chunks.iter_mut() {
      if chunk.tag == *tag {
        chunk.enabled = false;
      }
    }

    if let Some(focus) = self.active {
      self.set_focus(focus);
    }
  }

  pub fn disable_if<F>(&mut self, condition: F)
  where
    F: Fn(&T) -> bool,
  {
    for chunk in self.chunks.iter_mut() {
      if condition(&chunk.tag) {
        chunk.enabled = false;
      }
    }
  }

  pub fn handle_ev<E, I>(&self, ev: E, targets: I) -> Option<E>
  where
    E: 't,
    I: IntoIterator<Item = &'t mut dyn TaggedEventful<T, E>>,
  {
    let Some(focused_tag) = self.active.map(|idx| &self.chunks[idx].tag) else {
      return Some(ev);
    };
    targets
      .into_iter()
      .filter(|target| target.get_tag() == focused_tag)
      .try_fold(ev, |ev, target| target.handle_ev(ev))
  }
}

impl<T> FocusManager<T> {
  pub fn focus_style<S: Into<Style>>(self, style: S) -> Self {
    Self {
      focus_style: Some(style.into()),
      ..self
    }
  }

  pub fn unfocus_style<S: Into<Style>>(self, style: S) -> Self {
    Self {
      unfocus_style: Some(style.into()),
      ..self
    }
  }

  pub fn set_focus(&mut self, focus: usize) -> bool {
    if let Some((idx, _)) = self
      .chunks
      .iter()
      .enumerate()
      .filter(|(_, chunk)| chunk.enabled)
      .nth(focus)
    {
      self.active = Some(idx);
      true
    } else {
      false
    }
  }

  pub fn next(&mut self) -> bool {
    if let Some(idx) = self.active {
      for (new_idx, attempt) in self
        .chunks
        .iter()
        .enumerate()
        .cycle()
        .skip(idx + 1)
        .take(self.chunks.len())
      {
        if attempt.enabled {
          self.active = Some(new_idx);
          return true;
        }
      }
      self.active = None;
    }
    false
  }

  pub fn prev(&mut self) -> bool {
    if let Some(idx) = self.active {
      for (new_idx, attempt) in self
        .chunks
        .iter()
        .enumerate()
        .rev()
        .cycle()
        .skip(self.chunks.len() - idx)
        .take(self.chunks.len())
      {
        if attempt.enabled {
          self.active = Some(new_idx);
          return true;
        }
      }
      self.active = None;
    }
    false
  }
}

impl<T: Clone> FocusManager<T> {
  pub fn new_child(&mut self, tag: T) -> FocusableBuilder<T> {
    self.chunks.push(FocusChunk {
      enabled: true,
      tag: tag.clone(),
    });
    FocusableBuilder { tag }
  }

  pub fn new_hidden_child(&mut self, tag: T) -> FocusableBuilder<T> {
    self.chunks.push(FocusChunk {
      enabled: false,
      tag: tag.clone(),
    });
    FocusableBuilder { tag }
  }
}

struct FocusChunk<T> {
  enabled: bool,
  tag: T,
}

pub struct FocusableBuilder<T> {
  tag: T,
}

impl<T> FocusableBuilder<T> {
  pub fn with_state<S>(self, state: S) -> Focusable<T, S> {
    Focusable {
      tag: self.tag,
      state,
    }
  }
}

pub struct Focusable<T, S> {
  tag: T,
  pub state: S,
}

impl<T, S> Focusable<T, S> {
  pub fn into_inner(self) -> S {
    self.state
  }
}

impl<T: PartialEq, S> Focusable<T, S> {
  pub fn has_focus(&self, fmanager: &FocusManager<T>) -> bool {
    fmanager.is_focused(&self.tag)
  }

  pub fn widget<W: StylableWidget>(
    &self,
    mut widget: W,
    fmanager: &FocusManager<T>,
  ) -> W {
    if self.has_focus(fmanager) {
      widget.focus_style(fmanager.focus_style, true);
    } else {
      widget.focus_style(fmanager.unfocus_style, false);
    }
    widget
  }
}

impl<T: PartialEq, S: StylableWidget> Focusable<T, S> {
  pub fn internal_widget(&mut self, fmanager: &FocusManager<T>) -> &S {
    if self.has_focus(fmanager) {
      self.state.focus_style(fmanager.focus_style, true);
    } else {
      self.state.focus_style(fmanager.unfocus_style, false);
    }
    &self.state
  }
}

impl<T, S> std::ops::Deref for Focusable<T, S> {
  type Target = S;

  fn deref(&self) -> &Self::Target {
    &self.state
  }
}

impl<T, S> std::ops::DerefMut for Focusable<T, S> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.state
  }
}

pub trait TaggedState<T> {
  fn get_tag(&self) -> &T;
}

impl<T, S> TaggedState<T> for Focusable<T, S> {
  fn get_tag(&self) -> &T {
    &self.tag
  }
}

impl<E, T, S: EventfulState<E>> EventfulState<E> for Focusable<T, S> {
  fn handle_ev(&mut self, event: E) -> Option<E> {
    self.state.handle_ev(event)
  }
}

pub trait TaggedEventful<T, E>: TaggedState<T> + EventfulState<E> {}
impl<T, E, F: TaggedState<T> + EventfulState<E>> TaggedEventful<T, E> for F {}
