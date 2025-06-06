use color_eyre::eyre;
use itertools::Itertools;
use ratatui::{
  crossterm::event::{Event, KeyCode, KeyModifiers},
  layout::{Constraint, Layout, Margin},
  style::Color,
  text::{Line, Span},
  widgets::WidgetRef,
};
use uuid::Uuid;

use crate::{
  db::{format_word, WorkingDatabase},
  lang::Word,
};

use super::{
  traits::{EventfulState, WidgetRefMut},
  word_choice::{ChoiceWidget, ChoiceWidgetExit},
};

pub enum UpdateResult {
  Saved(Uuid, Word),
  Dismissed,
  Cancelled,
}

pub fn run_update<T: ratatui::prelude::Backend>(
  new_word: Word,
  old_words: Vec<Uuid>,
  db: &WorkingDatabase,
  terminal: &mut ratatui::Terminal<T>,
) -> eyre::Result<UpdateResult> {
  let new_word_text = format_word(db, &new_word);

  let shortcuts = Line::from_iter([
    Span::styled("<Enter>", Color::Blue),
    Span::raw(" to accept changes, "),
    Span::styled("C-c", Color::Blue),
    Span::raw(" to dismiss changes, "),
    Span::styled("C-x", Color::Blue),
    Span::raw(" to dismiss all remaining changes"),
  ]);

  let midbar = Line::raw("  old ^^^ vvv new  ");

  let old_words = old_words
    .into_iter()
    .map(|uuid| (uuid, &db.fetch(&uuid).unwrap().word))
    .collect_vec();

  let mut choice_widget = ChoiceWidget::new(
    old_words,
    |(_, w)| w.variants[0].as_str(),
    |(_, w)| format_word(db, &w),
  );

  loop {
    terminal.draw(|f| {
      let [choice_area, midbar_area, new_area, shortcuts_area] =
        Layout::vertical([
          Constraint::Fill(1),
          Constraint::Length(3),
          Constraint::Fill(1),
          Constraint::Length(1),
        ])
        .areas(f.area());

      choice_widget.render_ref_mut(choice_area, f.buffer_mut());

      midbar.render_ref(
        midbar_area.inner(Margin {
          horizontal: 0,
          vertical: 1,
        }),
        f.buffer_mut(),
      );

      new_word_text.render_ref(new_area, f.buffer_mut());

      shortcuts.render_ref(shortcuts_area, f.buffer_mut());
    })?;

    if let Event::Key(event) = ratatui::crossterm::event::read()? {
      let ev = choice_widget.handle_ev(event);
      if let Some(key_event!(KeyCode::Char('x'), KeyModifiers::CONTROL)) = ev {
        return Ok(UpdateResult::Cancelled);
      }

      match choice_widget.get_exit_status() {
        ChoiceWidgetExit::NoExit => {}
        ChoiceWidgetExit::Cancelled => return Ok(UpdateResult::Dismissed),
        ChoiceWidgetExit::Chosen => {
          let (uuid, _) = choice_widget.take_chosen();
          return Ok(UpdateResult::Saved(uuid, new_word));
        }
      }
    }
  }
}
