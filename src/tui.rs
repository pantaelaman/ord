use crate::parser::Commands;
use color_eyre::eyre;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use line_input::{LineInput, LineInputState};
use ratatui::{
  layout::{Constraint, Direction, Layout},
  style::Style,
  widgets::{Paragraph, Wrap},
};
use traits::{EventfulState, StylableWidget};

macro_rules! key_event {
  ($code:pat) => {
    KeyEvent { code: $code, .. }
  };
  ($code:pat, $modifier:pat) => {
    ::crossterm::event::KeyEvent {
      code: $code,
      modifiers: $modifier,
      ..
    }
  };
  ($code:pat, $modifier0:pat, $($modifiers:pat),+) => {
    key_event!($code, $modifier0) | key_event!($code, $($modifiers),+)
  };
}

mod dropdown;
mod focus;
mod left_label;
mod line_input;
mod traits;
mod word_edit;

pub fn handle_command(
  workingdb: &mut crate::db::WorkingDatabase,
  command: Commands,
) -> eyre::Result<String> {
  match command {
    Commands::Update {
      sheet_ty,
      sheet_file,
    } => crate::db::update(workingdb, sheet_ty, sheet_file),
    Commands::Dump => Ok(crate::db::dump(workingdb)),
    Commands::Phone { phone } => Ok(crate::db::phone(workingdb, phone)),
    Commands::Phones {
      popular_low,
      popular_high,
    } => Ok(crate::db::phones(workingdb, popular_low, popular_high)),
    Commands::Word { word, fuzzy } => {
      Ok(crate::db::word(workingdb, word, fuzzy))
    }
    Commands::Search { content } => {
      Ok(crate::db::search_english(workingdb, content))
    }
    Commands::Flags {
      ignore_terminal_Y,
      ignore_H,
    } => Ok(crate::db::set_flags(workingdb, ignore_terminal_Y, ignore_H)),
    _ => unimplemented!(),
  }
}

pub fn run(db: crate::db::Database) -> eyre::Result<crate::db::Database> {
  let mut workingdb: crate::db::WorkingDatabase = db.into();
  let mut terminal = ratatui::init();
  let mut command_input = LineInputState::default();
  let mut output = String::new();
  let mut scroll_y = 0;

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

      f.render_widget(
        Paragraph::new(output.clone())
          .wrap(Wrap { trim: false })
          .scroll((scroll_y, 0)),
        major,
      );
      f.render_widget(Paragraph::new(": "), prompt);
      f.render_stateful_widget(
        LineInput::default().style(Style::default()),
        cmd,
        &mut command_input,
      );
    })?;
    if let crossterm::event::Event::Key(k) = crossterm::event::read()? {
      match k {
        key_event!(KeyCode::Up) => {
          scroll_y = scroll_y.saturating_sub(1);
        }
        key_event!(KeyCode::Down) => {
          scroll_y = scroll_y.saturating_add(1);
        }
        key_event!(KeyCode::PageUp) => {
          scroll_y = scroll_y.saturating_sub(20);
        }
        key_event!(KeyCode::PageDown) => {
          scroll_y = scroll_y.saturating_add(20);
        }
        key_event!(KeyCode::Enter) => {
          // run command here
          match crate::parser::parse(command_input.get_str()) {
            Ok(Commands::Quit) => break,
            Ok(Commands::New) => {
              word_edit::run_word_edit(&mut workingdb, &mut terminal)?
            }
            Ok(c) => output = handle_command(&mut workingdb, c)?,
            Err(err) => output = format!("{err}"),
          }
          command_input.clear();
          scroll_y = 0;
        }
        ev => {
          command_input.handle_ev(ev);
        }
      }
    }
  }

  Ok(workingdb.into())
}
