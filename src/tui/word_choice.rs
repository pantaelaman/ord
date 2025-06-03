use color_eyre::eyre;
use ratatui::{
  crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers},
  layout::{Constraint, Layout},
  style::{Color, Style},
  text::Text,
  widgets::{
    List, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, StatefulWidget, Widget,
  },
};
use std::marker::PhantomData;

use super::traits::{EventfulState, WidgetRefMut};

#[derive(Clone, Copy, Default)]
pub enum ChoiceWidgetExit {
  #[default]
  NoExit,
  Chosen,
  Cancelled,
}

pub struct ChoiceWidget<'a, O: 'a, F, G, W>
where
  F: Fn(&O) -> &str,
  G: Fn(&O) -> W,
  W: Widget,
{
  options: Vec<O>,
  shorttext: F,
  longtext: G,
  _phantom: PhantomData<&'a O>,
  options_state: ListState,
  scrollbar_state: ScrollbarState,
  exit: ChoiceWidgetExit,
}

impl<'a, O: 'a, F, G, W> ChoiceWidget<'a, O, F, G, W>
where
  F: Fn(&O) -> &str,
  G: Fn(&O) -> W,
  W: Widget,
{
  pub fn new(options: Vec<O>, shorttext: F, longtext: G) -> Self {
    Self {
      scrollbar_state: ScrollbarState::new(std::cmp::max(
        options.len().saturating_sub(10),
        1,
      )),
      options,
      shorttext,
      longtext,
      _phantom: PhantomData,
      options_state: ListState::default().with_selected(Some(0)),
      exit: ChoiceWidgetExit::NoExit,
    }
  }

  pub fn get_exit_status(&self) -> ChoiceWidgetExit {
    self.exit
  }

  pub fn take_chosen(mut self) -> O {
    self
      .options
      .swap_remove(self.options_state.selected().unwrap())
  }
}

impl<'a, O: 'a, F, G, W> WidgetRefMut for ChoiceWidget<'a, O, F, G, W>
where
  F: Fn(&O) -> &str,
  G: Fn(&O) -> W,
  W: Widget,
{
  fn render_ref_mut(
    &mut self,
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
  ) {
    let [options_area, _, word_area] = Layout::vertical([
      Constraint::Length(10),
      Constraint::Length(1),
      Constraint::Fill(1),
    ])
    .areas(area);

    let [scrollbar_area, options_area] =
      Layout::horizontal([Constraint::Length(2), Constraint::Length(20)])
        .areas(options_area);

    let scrollbar_widget = Scrollbar::new(ScrollbarOrientation::VerticalLeft);
    scrollbar_widget.render(scrollbar_area, buf, &mut self.scrollbar_state);

    let options_widget = List::new(self.options.iter().map(&self.shorttext))
      .highlight_style(Style::new().bg(Color::Blue));

    StatefulWidget::render(
      options_widget,
      options_area,
      buf,
      &mut self.options_state,
    );

    (&self.longtext)(&self.options[self.options_state.selected().unwrap()])
      .render(word_area, buf);
  }
}

impl<'a, O: 'a, F, G, W> EventfulState<KeyEvent>
  for ChoiceWidget<'a, O, F, G, W>
where
  F: Fn(&O) -> &str,
  G: Fn(&O) -> W,
  W: Widget,
{
  fn handle_ev(&mut self, event: KeyEvent) -> Option<KeyEvent> {
    match event {
      key_event!(KeyCode::Up) => {
        self.options_state.scroll_up_by(1);
        self.scrollbar_state =
          self.scrollbar_state.position(self.options_state.offset());
      }
      key_event!(KeyCode::Down) => {
        self.options_state.scroll_down_by(1);
        self.scrollbar_state =
          self.scrollbar_state.position(self.options_state.offset());
      }
      key_event!(KeyCode::Enter) => self.exit = ChoiceWidgetExit::Chosen,
      key_event!(KeyCode::Char('c'), KeyModifiers::CONTROL) => {
        self.exit = ChoiceWidgetExit::Cancelled
      }
      ev => return Some(ev),
    }
    None
  }
}

pub fn run_choice<'a, T, O: 'a, F, G, W>(
  options: Vec<O>,
  shorttext: F,
  longtext: G,
  terminal: &mut ratatui::Terminal<T>,
) -> eyre::Result<Option<O>>
where
  T: ratatui::prelude::Backend,
  F: Fn(&O) -> &str,
  G: Fn(&O) -> W,
  W: Widget,
{
  let mut widget = ChoiceWidget::new(options, shorttext, longtext);

  loop {
    terminal.draw(|f| {
      widget.render_ref_mut(f.area(), f.buffer_mut());
    })?;

    if let Event::Key(event) = ratatui::crossterm::event::read()? {
      widget.handle_ev(event);
      match widget.get_exit_status() {
        ChoiceWidgetExit::NoExit => {}
        ChoiceWidgetExit::Chosen => break,
        ChoiceWidgetExit::Cancelled => return Ok(None),
      }
    }
  }

  Ok(Some(widget.take_chosen()))
}
