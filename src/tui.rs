use crate::db::{Database, WorkingDatabase};
use crate::parser::Commands;
use color_eyre::eyre;
use itertools::Itertools;
use line_input::LineInput;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::prelude::Backend;
use ratatui::style::Color;
use ratatui::text::Text;
use ratatui::widgets::{WidgetRef, Wrap};
use ratatui::Terminal;
use ratatui::{
  layout::{Constraint, Direction, Layout},
  widgets::Paragraph,
};
use scrollagraph::Scrollagraph;
use std::marker::PhantomData;
use traits::{EventfulState, WidgetRefMut};
use word_choice::{run_choice, ChoiceWidget};
use word_edit::run_word_edit;
use word_update::{run_update, UpdateResult};

macro_rules! key_event {
  ($code:pat) => {
    ::ratatui::crossterm::event::KeyEvent { code: $code, .. }
  };
  ($code:pat, $($modifier:pat),+) => {
    $(::ratatui::crossterm::event::KeyEvent {
      code: $code,
      modifiers: $modifier,
      ..
    })|+
  };
}

mod dropdown;
mod focus;
mod left_label;
mod line_input;
mod scrollagraph;
mod traits;
mod word_choice;
mod word_edit;
mod word_update;

fn handle_command<'a, B: Backend>(
  workingdb: &'a mut WorkingDatabase,
  terminal: &mut Terminal<B>,
  command: Commands,
) -> eyre::Result<State<'a>> {
  let text: Text<'a> = match command {
    Commands::Dump => crate::db::dump(workingdb),
    Commands::Phone {
      phone_matching,
      bounds,
    } => 'phone: {
      workingdb.get_phone_map();
      let mut phones = match crate::db::find_phone(
        workingdb.unwrap_phone_map(),
        phone_matching,
        bounds,
      ) {
        Ok(res) => res,
        Err(err) => {
          break 'phone Text::styled(format!("regex error {err}"), Color::Red)
        }
      };

      if phones.is_empty() {
        break 'phone Text::styled(
          format!("could not find any matching phones"),
          Color::Red,
        );
      }

      phones.sort_by_key(|(phone, _)| *phone);

      return Ok(State::new(ChoiceWidget::new(
        phones,
        |(phone, _)| phone,
        |(_, uuids)| {
          Paragraph::new(
            uuids
              .iter()
              .map(|uuid| &workingdb.fetch(uuid).unwrap().word.variants[0])
              .join(" "),
          )
          .wrap(Wrap { trim: false })
        },
      )));
    }
    Commands::Word {
      word,
      match_type,
      pos_guards,
      edit,
    } => 'word: {
      let variant_uuids = match workingdb
        .find_word(&word, match_type, pos_guards)
      {
        Ok(res) => res,
        Err(err) => {
          break 'word Text::styled(format!("regex error: {err}"), Color::Red);
        }
      };
      let mut variants = variant_uuids
        .into_iter()
        .map(|uuid| (uuid, workingdb.fetch(&uuid).unwrap()))
        .collect_vec();
      variants.sort_by_key(|(_, wd)| &wd.word.variants[0]);

      if variants.is_empty() {
        break 'word Text::styled(
          format!("found no words which matched {word}"),
          Color::Red,
        );
      }

      if !edit {
        return Ok(State::new(ChoiceWidget::new(
          variants
            .into_iter()
            .map(|(uuid, wd)| (uuid, wd.word.clone()))
            .collect_vec(),
          |(_, word)| word.variants[0].as_str(),
          |(_, word)| crate::db::format_word(workingdb, &word),
        )));
      } else {
        let Some((target, choice)) = run_choice(
          variants,
          |(_, wd)| wd.word.variants[0].as_str(),
          |(_, wd)| crate::db::format_word(&workingdb, &wd.word),
          terminal,
        )?
        else {
          break 'word Text::default();
        };

        let Some(updated) = run_word_edit(Some(&choice.word), terminal)? else {
          break 'word Text::default();
        };

        workingdb.update_word(target, updated);
        crate::db::format_word(
          &workingdb,
          &workingdb.fetch(&target).unwrap().word,
        )
      }
    }
    Commands::New => 'new: {
      let Some(word) = run_word_edit(None, terminal)? else {
        break 'new Text::default();
      };

      let id = workingdb.new_word(word);
      crate::db::format_word(&workingdb, &workingdb.fetch(&id).unwrap().word)
    }
    Commands::Choose => {
      let choices = workingdb
        .debug_get_n(12)
        .into_iter()
        .map(|uuid| (uuid, workingdb.fetch(&uuid).unwrap()))
        .collect_vec();
      let _ = run_choice(
        choices,
        |(_, wd)| wd.word.variants[0].as_str(),
        |(_, wd)| crate::db::format_word(&workingdb, &wd.word),
        terminal,
      )?;
      Text::default()
    }
    Commands::Search { content } => {
      crate::db::search_english(workingdb, content)
    }
    Commands::Flags {
      ignore_terminal_Y,
      ignore_H,
    } => crate::db::set_flags(workingdb, ignore_terminal_Y, ignore_H),
    Commands::Update {
      sheet_ty,
      sheet_file,
    } => {
      let result = crate::db::auto_update(workingdb, sheet_ty, sheet_file)?;

      let mut conflicts_saved = 0;
      let mut conflicts_skipped = 0;
      let mut conflicts_iter = result.conflicts.into_iter();

      while let Some((word, uuids)) = conflicts_iter.next() {
        match run_update(word, uuids, workingdb, terminal)? {
          UpdateResult::Saved(uuid, word) => {
            workingdb.update_word(uuid, word);
            conflicts_saved += 1;
          }
          UpdateResult::Dismissed => conflicts_skipped += 1,
          UpdateResult::Cancelled => break,
        }
      }

      conflicts_skipped += conflicts_iter.count(); // drain any remaining conflicts

      Text::from_iter([
        format!("Added {} new records", result.added),
        format!("Skipped {} duplicate records", result.skipped),
        format!("Overwrote {} conflicting records", conflicts_saved),
        format!("Skipped {} conflicting records", conflicts_skipped),
      ])
    }
    _ => unimplemented!(),
  };

  Ok(State::new_scrollagraph(text))
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EmptyWidget {}

impl WidgetRef for EmptyWidget {
  fn render_ref(&self, _area: Rect, _buf: &mut Buffer) {}
}

impl<E> EventfulState<E> for EmptyWidget {
  fn handle_ev(&mut self, event: E) -> Option<E> {
    Some(event)
  }
}

trait MainviewWidget<'a>: WidgetRefMut + EventfulState<KeyEvent> + 'a {}
impl<'a, T: WidgetRefMut + EventfulState<KeyEvent> + 'a> MainviewWidget<'a>
  for T
{
}

struct State<'a> {
  main_widget: Box<dyn MainviewWidget<'a>>,
  _phantom: PhantomData<&'a dyn MainviewWidget<'a>>,
}

impl<'a> State<'a> {
  fn new_empty() -> Self {
    Self {
      main_widget: Box::new(EmptyWidget {}),
      _phantom: PhantomData,
    }
  }

  fn new_scrollagraph<T: Into<Text<'a>>>(content: T) -> Self {
    Self {
      main_widget: Box::new(Scrollagraph::new(content)),
      _phantom: PhantomData,
    }
  }

  fn new<W: MainviewWidget<'a>>(child: W) -> Self {
    Self {
      main_widget: Box::new(child),
      _phantom: PhantomData,
    }
  }
}

impl<'a> WidgetRefMut for State<'a> {
  fn render_ref_mut(&mut self, area: Rect, buf: &mut Buffer) {
    self.main_widget.render_ref_mut(area, buf);
  }
}

impl<'a> EventfulState<KeyEvent> for State<'a> {
  fn handle_ev(&mut self, event: KeyEvent) -> Option<KeyEvent> {
    self.main_widget.handle_ev(event)
  }
}

pub fn run(db: Database) -> eyre::Result<crate::db::Database> {
  let mut workingdb: WorkingDatabase = db.into();
  let mut state = State::new_empty();
  let mut terminal = ratatui::init();
  let mut command_input = LineInput::default_area();

  loop {
    terminal.draw(|f| {
      let [major, cmd] = Layout::new(
        Direction::Vertical,
        [Constraint::Fill(1), Constraint::Length(1)],
      )
      .areas(f.area());
      let [prompt, cmd] = Layout::new(
        Direction::Horizontal,
        [Constraint::Length(2), Constraint::Fill(1)],
      )
      .areas(cmd);

      state.render_ref_mut(major, f.buffer_mut());

      f.render_widget(Paragraph::new(": "), prompt);
      f.render_widget(&command_input, cmd);
    })?;
    if let ratatui::crossterm::event::Event::Key(k) =
      ratatui::crossterm::event::read()?
    {
      match k {
        key_event!(KeyCode::Enter) => {
          // run command here
          match crate::parser::parse(
            &std::mem::replace(&mut command_input, LineInput::default_area())
              .text_area
              .into_lines()[0],
          ) {
            Ok(Commands::Quit) => break,
            Ok(c) => {
              std::mem::drop(state);
              state = handle_command(&mut workingdb, &mut terminal, c)?
            }
            Err(err) => {
              state = State::new_scrollagraph(Text::styled(
                format!("{err}"),
                Color::Red,
              ))
            }
          }
        }
        ev => {
          if let Some(ev) = state.handle_ev(ev) {
            command_input.handle_ev(ev);
          }
        }
      }
    }
  }

  std::mem::drop(state);
  Ok(workingdb.into())
}
