use std::marker::PhantomData;

use color_eyre::eyre;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
  buffer::Buffer,
  layout::{Constraint, Layout, Rect},
  style::{Color, Modifier, Style},
  text::Line,
  widgets::StatefulWidget,
};

use crate::lang::POS;

use super::{
  dropdown::{Dropdown, DropdownState},
  focus::{FocusManager, Focusable, FocusableState, Focuser, NamedFocusChain},
  left_label::LeftLabelled,
  line_input::{LineInput, LineInputState},
  traits::{EventfulState, OptionEventExt},
};

macro_rules! wedit_fmanager {
  () => {
    FocusManager::default()
      .focus_style(Modifier::UNDERLINED)
      .unfocus_style(
        Style::default()
          .fg(Color::Blue)
          .add_modifier(Modifier::BOLD),
      )
  };
}

macro_rules! wedit_dropdown {
  ($state:expr) => {
    Dropdown::new($state)
      .selected_style(Style::reset().fg(Color::Black).bg(Color::Yellow))
      .open_style(Style::reset().fg(Color::Black).bg(Color::Blue));
  };
}

macro_rules! wedit_lineinput {
  ($state:expr, $label:expr) => {
    Focusable::new(
      $state,
      LeftLabelled::new($label, LineInput::default().strict_width(20)),
    )
  };
}

pub fn run_word_edit<T: ratatui::prelude::Backend>(
  db: &mut crate::db::WorkingDatabase,
  terminal: &mut ratatui::Terminal<T>,
) -> eyre::Result<()> {
  let mut top_focus_manager = wedit_fmanager!();
  let mut variants_input: FocusableState<LineInputState> =
    top_focus_manager.new_child(LineInputState::default());
  let mut pos_input: FocusableState<DropdownState<&'static str>> =
    top_focus_manager.new_child(DropdownState::new([
      "noun",
      "particle",
      "descriptor",
      "verb",
    ]));
  let mut bot_focus_manager = wedit_fmanager!();
  let mut phones_input: FocusableState<LineInputState> =
    bot_focus_manager.new_child(LineInputState::default());
  let mut definition_input: FocusableState<LineInputState> =
    bot_focus_manager.new_child(LineInputState::default());

  let mut focus_chain = NamedFocusChain::new([
    ("top", top_focus_manager),
    ("bot", bot_focus_manager),
  ]);

  focus_chain.register_focuser("pos", wedit_fmanager!());

  focus_chain.grant_focus();
  let (mut pos_component, _) = POS_Component::from_dropdown(pos_input.get());

  loop {
    if pos_input.changed() {
      focus_chain.remove_focuser(&"pos");
      let (posc, fmanager) = POS_Component::from_dropdown(pos_input.get());
      pos_component = posc;
      *focus_chain.get_focuser_mut(&"pos").unwrap() = fmanager;
      if let POS_Component::Particle { .. } = pos_component {
        focus_chain.insert_focuser_after("pos", &"top");
      } else {
        focus_chain.insert_focuser_after("pos", &"bot");
      }
    }

    terminal.draw(|f| {
        let [
          variants_line,
          pos_line,
          _,
          phones_line,
          _,
          definition_lines,
          _,
          pos_specific_chunk,
          ] = Layout::vertical([
          Constraint::Length(1),
          Constraint::Length(1),
          Constraint::Length(1),
          Constraint::Length(1),
          Constraint::Length(1),
          Constraint::Length(3),
          Constraint::Length(1),
          Constraint::Fill(1),
        ]).areas(f.area());

        let variants_widget = wedit_lineinput!(&mut variants_input, "Variants: ");
        let pos_widget = wedit_dropdown!(&mut pos_input);
        let phones_widget = wedit_lineinput!(&mut phones_input, "Phones: ");
        let definition_widget = wedit_lineinput!(&mut definition_input, "Definition: ");

        let [
          pos_seg,
          _,
          cat_seg
          ] = Layout::horizontal([
            Constraint::Length(pos_widget.line_width()),
            Constraint::Length(2),
            Constraint::Fill(1)
          ]).areas(pos_line);

        let mut top_focuser = focus_chain.get_focuser(&"top").unwrap();
        let mut bot_focuser = focus_chain.get_focuser(&"bot").unwrap();

        f.render_stateful_widget(variants_widget, variants_line, &mut top_focuser);
        f.render_stateful_widget(phones_widget, phones_line, &mut bot_focuser);
        f.render_stateful_widget(definition_widget, definition_lines, &mut bot_focuser);
        f.render_stateful_widget(pos_widget, pos_seg, &mut top_focuser);

        pos_component.render(f.buffer_mut(), pos_specific_chunk, cat_seg, focus_chain.get_focuser(&"pos").unwrap());

        //f.render_widget(
        //  Paragraph::new(
        //    format!(
        //      "{:?} {}:{}:{}",
        //      main_focus_manager.focus,
        //      variants_input.focus_id,
        //      phones_input.focus_id,
        //      definition_input.focus_id)),
        //    pos_line);
      })?;

    if let crossterm::event::Event::Key(k) = crossterm::event::read()? {
      match k {
        key_event!(KeyCode::Char('c'), KeyModifiers::CONTROL) => {
          break;
        }
        key_event!(KeyCode::Tab, KeyModifiers::NONE) => {
          focus_chain.focus_next();
        }
        ev => {
          let top_focuser = focus_chain.get_focuser(&"top").unwrap();
          let bot_focuser = focus_chain.get_focuser(&"bot").unwrap();
          let pos_focuser = focus_chain.get_focuser(&"pos").unwrap();

          (&mut variants_input, top_focuser)
            .handle_ev(ev)
            .chain_with(&mut (&mut pos_input, top_focuser))
            .chain_with(&mut (&mut phones_input, bot_focuser))
            .chain_with(&mut (&mut definition_input, bot_focuser))
            .chain_with(&mut (&mut pos_component, pos_focuser));
        }
      }
    }
  }

  Ok(())
}

#[allow(non_camel_case_types)]
enum POS_Component {
  Noun {
    plural: FocusableState<LineInputState>,
    definite: FocusableState<LineInputState>,
    definite_plural: FocusableState<LineInputState>,
  },
  Descriptor {
    plural: FocusableState<LineInputState>,
    adverbial: FocusableState<LineInputState>,
  },
  Verb {
    present: FocusableState<LineInputState>,
    past: FocusableState<LineInputState>,
  },
  Particle {
    dropdown: FocusableState<DropdownState<&'static str>>,
  },
}

impl POS_Component {
  fn from_dropdown(pos: &'static str) -> (Self, FocusManager) {
    let mut fmanager = wedit_fmanager!();
    (
      match pos {
        "noun" => Self::Noun {
          plural: fmanager.new_child(LineInputState::default()),
          definite: fmanager.new_child(LineInputState::default()),
          definite_plural: fmanager.new_child(LineInputState::default()),
        },
        "descriptor" => Self::Descriptor {
          plural: fmanager.new_child(LineInputState::default()),
          adverbial: fmanager.new_child(LineInputState::default()),
        },
        "verb" => Self::Verb {
          present: fmanager.new_child(LineInputState::default()),
          past: fmanager.new_child(LineInputState::default()),
        },
        "particle" => Self::Particle {
          dropdown: fmanager.new_child(DropdownState::new([
            "article",
            "comparator",
            "conjunction",
            "contextual",
            "discursive",
            "indicator",
            "introductor",
            "miscellaneous",
          ])),
        },
        _ => unimplemented!(),
      },
      fmanager,
    )
  }

  fn render<'a>(
    &'a mut self,
    buf: &mut Buffer,
    pos_area: Rect,
    cat_area: Rect,
    mut fmanager: &'a FocusManager,
  ) {
    const OFFSET: usize = 14;

    match self {
      Self::Noun {
        plural,
        definite,
        definite_plural,
      } => {
        let [plural_line, definite_line, definite_plural_line, _] =
          Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
          ])
          .areas(pos_area);

        let plural_widget = wedit_lineinput!(
          plural,
          format!("{:<width$}", "Plural: ", width = OFFSET)
        );
        let definite_widget = wedit_lineinput!(
          definite,
          format!("{:<width$}", "Definite: ", width = OFFSET)
        );
        let definite_plural_widget = wedit_lineinput!(
          definite_plural,
          format!("{:<width$}", "Def. Plural: ", width = OFFSET)
        );

        plural_widget.render(plural_line, buf, &mut fmanager);
        definite_widget.render(definite_line, buf, &mut fmanager);
        definite_plural_widget.render(definite_plural_line, buf, &mut fmanager);
      }
      Self::Descriptor { plural, adverbial } => {
        let [plural_line, adverbial_line, _] = Layout::vertical([
          Constraint::Length(1),
          Constraint::Length(1),
          Constraint::Fill(1),
        ])
        .areas(pos_area);

        let plural_widget = wedit_lineinput!(
          plural,
          format!("{:<width$}", "Plural: ", width = OFFSET)
        );
        let adverbial_widget = wedit_lineinput!(
          adverbial,
          format!("{:<width$}", "Adverbial: ", width = OFFSET)
        );

        plural_widget.render(plural_line, buf, &mut fmanager);
        adverbial_widget.render(adverbial_line, buf, &mut fmanager);
      }
      Self::Verb { present, past } => {
        let [present_line, past_line, _] = Layout::vertical([
          Constraint::Length(1),
          Constraint::Length(1),
          Constraint::Fill(1),
        ])
        .areas(pos_area);

        let present_widget = wedit_lineinput!(
          present,
          format!("{:<width$}", "Present: ", width = OFFSET)
        );
        let past_widget = wedit_lineinput!(
          past,
          format!("{:<width$}", "Past: ", width = OFFSET)
        );

        present_widget.render(present_line, buf, &mut fmanager);
        past_widget.render(past_line, buf, &mut fmanager);
      }
      Self::Particle { dropdown } => {
        let cat_widget = wedit_dropdown!(dropdown);

        let cat_area = cat_widget.limit_area_width(cat_area);
        cat_widget.render(cat_area, buf, &mut fmanager);
      }
    }
  }
}

impl EventfulState<KeyEvent> for (&mut POS_Component, &FocusManager) {
  fn handle_ev(&mut self, event: KeyEvent) -> Option<KeyEvent> {
    let (posc, fmanager) = self;
    match posc {
      POS_Component::Noun {
        plural,
        definite,
        definite_plural,
      } => (plural, *fmanager)
        .handle_ev(event)
        .chain_with(&mut (definite, *fmanager))
        .chain_with(&mut (definite_plural, *fmanager)),
      POS_Component::Descriptor { plural, adverbial } => (plural, *fmanager)
        .handle_ev(event)
        .chain_with(&mut (adverbial, *fmanager)),
      POS_Component::Verb { present, past } => (present, *fmanager)
        .handle_ev(event)
        .chain_with(&mut (past, *fmanager)),
      POS_Component::Particle { dropdown } => {
        (dropdown, *fmanager).handle_ev(event)
      }
    }
  }
}
