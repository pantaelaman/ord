use std::{
  collections::{HashMap, VecDeque},
  hash::Hash,
};

use ratatui::{style::Style, widgets::StatefulWidget};

use super::traits::{EventfulState, StylableWidget};

pub trait Focuser {
  fn grant_focus(&mut self);
  fn grant_focus_end(&mut self);
  fn revoke_focus(&mut self);
  fn focus_next(&mut self) -> bool;
  fn focus_prev(&mut self) -> bool;
  fn set_focus(&mut self, focus: usize);
}

#[derive(Default)]
pub struct NamedFocusChain<T: Hash + Eq + Clone, F> {
  focusers: VecDeque<T>,
  focuser_map: HashMap<T, F>,
  focus: Option<usize>,
}

impl<T: Hash + Eq + Clone, F> NamedFocusChain<T, F> {
  pub fn new<I: IntoIterator<Item = (T, F)>>(focusers: I) -> Self {
    let mut order = VecDeque::new();
    let mut map = HashMap::new();
    for (name, focuser) in focusers {
      order.push_back(name.clone());
      map.insert(name, focuser);
    }
    NamedFocusChain {
      focusers: order,
      focuser_map: map,
      focus: None,
    }
  }

  pub fn register_focuser(&mut self, key: T, focuser: F) -> Option<F> {
    self.focuser_map.insert(key, focuser)
  }

  pub fn insert_focuser_after(&mut self, key: T, after: &T) -> bool {
    self
      .focusers
      .iter()
      .position(|k| k == after)
      .map(|idx| self.focusers.insert(idx + 1, key))
      .is_some()
  }

  pub fn insert_focuser_at(&mut self, key: T, index: usize) {
    self.focusers.insert(index, key);
  }

  pub fn get_focuser(&self, key: &T) -> Option<&F> {
    self.focuser_map.get(key)
  }

  pub fn get_focuser_mut(&mut self, key: &T) -> Option<&mut F> {
    self.focuser_map.get_mut(key)
  }

  pub fn get_focuser_index(&mut self, index: usize) -> Option<&mut F> {
    self
      .focusers
      .get(index)
      .cloned()
      .and_then(|key| self.get_focuser_mut(&key))
  }

  pub fn remove_focuser(&mut self, key: &T) {
    self.focusers.retain(|k| k != key);
  }
}

impl<T: Hash + Eq + Clone, F: Focuser> NamedFocusChain<T, F> {
  fn recalc_focus(&mut self) {
    if let Some(focus) = self.focus {
      self.set_focus(focus);
    }
  }
}

impl<T: Hash + Eq + Clone, F: Focuser> Focuser for NamedFocusChain<T, F> {
  fn grant_focus(&mut self) {
    if !self.focusers.is_empty() {
      self.set_focus(0);
    }
  }

  fn grant_focus_end(&mut self) {
    if !self.focusers.is_empty() {
      self.set_focus(self.focusers.len() - 1);
    }
  }

  fn revoke_focus(&mut self) {
    self.focus = None;
  }

  fn focus_next(&mut self) -> bool {
    if let Some(focus) = self.focus {
      let focuser = self.get_focuser_index(focus).unwrap();
      if focuser.focus_next() {
        focuser.revoke_focus();
        *self.focus.as_mut().unwrap() += 1;
        let res = self.focus.unwrap() > self.focusers.len();
        *self.focus.as_mut().unwrap() %= self.focusers.len();
        self
          .get_focuser_index(self.focus.unwrap())
          .unwrap()
          .grant_focus();
        res
      } else {
        false
      }
    } else {
      true
    }
  }

  fn focus_prev(&mut self) -> bool {
    if let Some(focus) = self.focus {
      let focuser = self.get_focuser_index(focus).unwrap();
      if focuser.focus_prev() {
        focuser.revoke_focus();
        let res = if focus > 0 {
          *self.focus.as_mut().unwrap() -= 1;
          false
        } else {
          *self.focus.as_mut().unwrap() = self.focusers.len() - 1;
          true
        };
        self
          .get_focuser_index(self.focus.unwrap())
          .unwrap()
          .grant_focus_end();
        res
      } else {
        false
      }
    } else {
      true
    }
  }

  fn set_focus(&mut self, focus: usize) {
    self.focus = Some(focus % self.focusers.len());
    self
      .get_focuser_index(self.focus.unwrap())
      .map(|f| f.grant_focus());
  }
}

#[derive(Default, Debug)]
pub struct FocusManager {
  next_id: usize,
  pub focus: Option<usize>,
  focus_style: Option<Style>,
  unfocus_style: Option<Style>,
}

impl FocusManager {
  pub fn focus_style<S: Into<Style>>(self, style: S) -> Self {
    FocusManager {
      focus_style: Some(style.into()),
      ..self
    }
  }

  pub fn unfocus_style<S: Into<Style>>(self, style: S) -> Self {
    FocusManager {
      unfocus_style: Some(style.into()),
      ..self
    }
  }

  pub fn new_child<S>(&mut self, child_state: S) -> FocusableState<S> {
    let state = FocusableState {
      focus_id: self.next_id,
      focus_style: self.focus_style.clone(),
      unfocus_style: self.unfocus_style.clone(),
      child_state,
    };
    self.next_id += 1;
    state
  }
}

impl Focuser for FocusManager {
  fn grant_focus(&mut self) {
    if self.next_id > 0 {
      self.focus = Some(0);
    }
  }

  fn grant_focus_end(&mut self) {
    if self.next_id > 0 {
      self.focus = Some(self.next_id - 1);
    }
  }

  fn revoke_focus(&mut self) {
    self.focus = None;
  }

  fn focus_next(&mut self) -> bool {
    if let Some(f) = self.focus.as_mut() {
      *f += 1;
      let res = *f >= self.next_id;
      *f %= self.next_id;
      res
    } else {
      true
    }
  }

  fn focus_prev(&mut self) -> bool {
    if let Some(f) = self.focus.as_mut() {
      if *f == 0 {
        *f = self.next_id - 1;
        true
      } else {
        *f -= 1;
        false
      }
    } else {
      true
    }
  }

  fn set_focus(&mut self, focus: usize) {
    self.focus = Some(focus % self.next_id);
  }
}

pub struct FocusableState<S> {
  pub focus_style: Option<Style>,
  pub unfocus_style: Option<Style>,
  pub focus_id: usize,
  child_state: S,
}

impl<S> std::ops::Deref for FocusableState<S> {
  type Target = S;

  fn deref(&self) -> &Self::Target {
    &self.child_state
  }
}

impl<S> std::ops::DerefMut for FocusableState<S> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.child_state
  }
}

impl<E, S: EventfulState<E>> EventfulState<E>
  for (&mut FocusableState<S>, &FocusManager)
{
  fn handle_ev(&mut self, event: E) -> Option<E> {
    let (fstate, fmanager) = self;

    if fmanager.focus.is_some_and(|focus| fstate.focus_id == focus) {
      fstate.child_state.handle_ev(event)
    } else {
      Some(event)
    }
  }
}

pub struct Focusable<'a, W: StatefulWidget> {
  widget: W,
  state: &'a mut FocusableState<<W as StatefulWidget>::State>,
}

impl<'a, W: StatefulWidget> Focusable<'a, W> {
  pub fn new(
    state: &'a mut FocusableState<<W as StatefulWidget>::State>,
    widget: W,
  ) -> Self {
    Focusable { widget, state }
  }
}

impl<'a, W: StatefulWidget + StylableWidget> StatefulWidget
  for Focusable<'a, W>
{
  type State = &'a FocusManager;

  fn render(
    self,
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
    fmanager: &mut Self::State,
  ) {
    let widget = if fmanager.focus.is_some_and(|f| self.state.focus_id == f) {
      self.widget.focus_style(self.state.focus_style, true)
    } else {
      self.widget.focus_style(self.state.unfocus_style, false)
    };

    widget.render(area, buf, &mut self.state.child_state);
  }
}
