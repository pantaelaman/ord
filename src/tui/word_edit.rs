use color_eyre::eyre;
use itertools::Itertools;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
  buffer::Buffer,
  layout::{Constraint, Layout, Rect},
  style::{Color, Style},
  widgets::{StatefulWidget, Widget},
};
use tui_textarea::TextArea;

use crate::lang::{self, POSType, ParticleCategory, Word};

use super::line_input::LineInput;
use super::{
  dropdown::{Dropdown, DropdownState},
  focus::{FocusManager, Focusable},
  left_label::LeftLabelled,
};

const LEFT_OFFSET: u16 = 15;

macro_rules! wedit_fmanager {
  () => {
    FocusManager::default()
      .focus_style(Style::default())
      .unfocus_style(Style::new().fg(Color::Blue))
  };
}

macro_rules! wedit_dropdown {
  () => {
    Dropdown::default()
      .selected_style(Style::reset().fg(Color::Black).bg(Color::Yellow))
      .open_style(Style::reset().fg(Color::Black).bg(Color::Blue))
  };
}

macro_rules! wedit_lineinput {
  ($focus:expr, $fmanager:expr, $label:expr) => {
    LeftLabelled::new($label, $focus.internal_widget(&$fmanager))
      .width(LEFT_OFFSET)
  };
}

macro_rules! wedit_evhandlers {
  ($($handler:expr),+) => {
    [$($handler as &mut dyn crate::tui::focus::TaggedEventful<_, _>),+]
  }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
enum ComponentTag {
  Variants,
  POS,
  Phones,
  Definition,
  Category,
  Inp1,
  Inp2,
  Inp3,
}

pub fn run_word_edit<T: ratatui::prelude::Backend>(
  word: Option<&Word>,
  terminal: &mut ratatui::Terminal<T>,
) -> eyre::Result<Option<Word>> {
  let mut fmanager = wedit_fmanager!();
  let mut variants_input: Focusable<ComponentTag, LineInput> = fmanager
    .new_child(ComponentTag::Variants)
    .with_state(LineInput::default_or_with(
      word.map(|w| w.variants.clone().into_iter().join(",")),
    ));
  let mut pos_input: Focusable<ComponentTag, DropdownState<POSType>> =
    fmanager.new_child(ComponentTag::POS).with_state(
      DropdownState::new([
        POSType::Noun,
        POSType::Descriptor,
        POSType::Verb,
        POSType::Particle,
      ])
      .with_initial(|o| word.is_none_or(|word| word.pos.ty() == *o)),
    );

  fmanager.new_hidden_child(ComponentTag::Category);

  let mut phones_input: Focusable<ComponentTag, LineInput> = fmanager
    .new_child(ComponentTag::Phones)
    .with_state(LineInput::default_or_with(
      word.map(|w| w.phones.clone().into_iter().join(",")),
    ));
  let mut definition_input: Focusable<ComponentTag, TextArea> = fmanager
    .new_child(ComponentTag::Definition)
    .with_state(match word {
      Some(ref w) => {
        TextArea::new(w.definition.lines().map(|s| s.to_owned()).collect_vec())
      }
      None => TextArea::default(),
    });

  definition_input.set_cursor_line_style(Style::default());

  fmanager.new_hidden_child(ComponentTag::Inp1);
  fmanager.new_hidden_child(ComponentTag::Inp2);
  fmanager.new_hidden_child(ComponentTag::Inp3);

  let mut pos_component = match word {
    Some(w) => POS_Component::from_word(w, &mut fmanager),
    None => POS_Component::from_dropdown(*pos_input.get(), &mut fmanager),
  };

  fmanager.set_focus(0);

  if word.is_some_and(|word| !word.is_standard()) {
    // reset changed so we don't overwrite the irregular forms
    pos_input.changed();
  }

  loop {
    if pos_input.changed() {
      fmanager.disable_if(|t| {
        [
          ComponentTag::Inp1,
          ComponentTag::Inp2,
          ComponentTag::Inp3,
          ComponentTag::Category,
        ]
        .contains(t)
      });
      pos_component =
        POS_Component::from_dropdown(*pos_input.get(), &mut fmanager);
      pos_component.update_placeholders(variants_input.lines()[0].as_str());
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

        f.render_widget(wedit_lineinput!(variants_input, fmanager, "Word: "), variants_line);
        f.render_widget(wedit_lineinput!(phones_input, fmanager, "Phones: "), phones_line);
        f.render_widget(wedit_lineinput!(definition_input, fmanager, "Definition: "), definition_lines);

        let [
          pos_seg,
          _,
          cat_seg
          ] = Layout::horizontal([
            Constraint::Length(pos_input.line_width() + 14),
            Constraint::Length(2),
            Constraint::Fill(1)
          ]).areas(pos_line);

        f.render_stateful_widget(
          pos_input.widget(
            LeftLabelled::new("POS: ", wedit_dropdown!()).width(LEFT_OFFSET),
            &fmanager
          ),
          pos_seg,
          &mut pos_input
        );

        pos_component.render(f.buffer_mut(), pos_specific_chunk, cat_seg, &fmanager);
      })?;

    if let ratatui::crossterm::event::Event::Key(k) =
      ratatui::crossterm::event::read()?
    {
      match k {
        key_event!(KeyCode::Char('c'), KeyModifiers::CONTROL) => {
          return Ok(None);
        }
        key_event!(KeyCode::Char('s'), KeyModifiers::CONTROL) => {
          break;
        }
        key_event!(KeyCode::Tab, KeyModifiers::NONE) => {
          fmanager.next();
        }
        key_event!(KeyCode::BackTab, KeyModifiers::SHIFT) => {
          fmanager.prev();
        }
        ev => {
          if variants_input.has_focus(&fmanager) {
            if variants_input.input(ev) {
              pos_component
                .update_placeholders(variants_input.lines()[0].as_str());
            }
          }

          if let Some(ev) = fmanager.handle_ev(
            ev,
            wedit_evhandlers!(
              &mut pos_input,
              &mut phones_input,
              &mut definition_input
            ),
          ) {
            pos_component.handle_ev(&fmanager, ev);
          }
        }
      }
    }
  }

  let variants = Into::<TextArea>::into(variants_input.into_inner())
    .into_lines()
    .into_iter()
    .nth(0)
    .unwrap()
    .split(",")
    .map(|s| s.to_owned())
    .collect_vec();
  let phones = Into::<TextArea>::into(phones_input.into_inner())
    .into_lines()
    .into_iter()
    .nth(0)
    .unwrap()
    .split(",")
    .map(|s| s.to_owned())
    .collect_vec();
  let definition = definition_input
    .into_inner()
    .into_lines()
    .into_iter()
    .filter(|s| !s.is_empty())
    .join("; ");
  let (irregular, pos) = pos_component.into_pos(&variants[0]);

  Ok(Some(Word {
    variants,
    phones,
    definition,
    standard: !irregular,
    pos,
  }))
}

#[allow(non_camel_case_types)]
enum POS_Component<'a> {
  Noun {
    plural: Focusable<ComponentTag, LineInput<'a>>,
    definite: Focusable<ComponentTag, LineInput<'a>>,
    definite_plural: Focusable<ComponentTag, LineInput<'a>>,
  },
  Descriptor {
    plural: Focusable<ComponentTag, LineInput<'a>>,
    adverbial: Focusable<ComponentTag, LineInput<'a>>,
  },
  Verb {
    present: Focusable<ComponentTag, LineInput<'a>>,
    past: Focusable<ComponentTag, LineInput<'a>>,
  },
  Particle {
    dropdown: Focusable<ComponentTag, DropdownState<ParticleCategory>>,
  },
}

macro_rules! expand_generic {
  ($irregular:ident, $input:expr, $generator:expr) => {{
    let val = Into::<TextArea>::into($input)
      .into_lines()
      .into_iter()
      .nth(0)
      .unwrap();
    if val.is_empty() {
      $generator
    } else {
      $irregular = true;
      val
    }
  }};
}

impl<'a> POS_Component<'a> {
  fn from_dropdown(
    pos: POSType,
    fmanager: &mut FocusManager<ComponentTag>,
  ) -> Self {
    match pos {
      POSType::Noun => Self::Noun {
        plural: fmanager
          .new_from_existing(ComponentTag::Inp1)
          .with_state(LineInput::default()),
        definite: fmanager
          .new_from_existing(ComponentTag::Inp2)
          .with_state(LineInput::default()),
        definite_plural: fmanager
          .new_from_existing(ComponentTag::Inp3)
          .with_state(LineInput::default()),
      },
      POSType::Descriptor => Self::Descriptor {
        plural: fmanager
          .new_from_existing(ComponentTag::Inp1)
          .with_state(LineInput::default()),
        adverbial: fmanager
          .new_from_existing(ComponentTag::Inp2)
          .with_state(LineInput::default()),
      },
      POSType::Verb => Self::Verb {
        present: fmanager
          .new_from_existing(ComponentTag::Inp1)
          .with_state(LineInput::default()),
        past: fmanager
          .new_from_existing(ComponentTag::Inp2)
          .with_state(LineInput::default()),
      },
      POSType::Particle => Self::Particle {
        dropdown: fmanager
          .new_from_existing(ComponentTag::Category)
          .with_state(DropdownState::new([
            ParticleCategory::Miscellaneous,
            ParticleCategory::Article,
            ParticleCategory::Comparator,
            ParticleCategory::Conjunction,
            ParticleCategory::Contextual,
            ParticleCategory::Discursive,
            ParticleCategory::Indicator,
            ParticleCategory::Introductor,
          ])),
      },
    }
  }

  fn from_word(word: &Word, fmanager: &mut FocusManager<ComponentTag>) -> Self {
    match &word.pos {
      lang::POS::Noun {
        plural,
        definite,
        definite_plural,
      } => Self::Noun {
        plural: fmanager.new_from_existing(ComponentTag::Inp1).with_state(
          LineInput::default_or_with((!word.standard).then(|| plural.clone())),
        ),
        definite: fmanager.new_from_existing(ComponentTag::Inp2).with_state(
          LineInput::default_or_with(
            (!word.standard).then(|| definite.clone()),
          ),
        ),
        definite_plural: fmanager
          .new_from_existing(ComponentTag::Inp3)
          .with_state(LineInput::default_or_with(
            (!word.standard).then(|| definite_plural.clone()),
          )),
      },
      lang::POS::Descriptor { plural, adverbial } => Self::Descriptor {
        plural: fmanager.new_from_existing(ComponentTag::Inp1).with_state(
          LineInput::default_or_with((!word.standard).then(|| plural.clone())),
        ),
        adverbial: fmanager.new_from_existing(ComponentTag::Inp2).with_state(
          LineInput::default_or_with(
            (!word.standard).then(|| adverbial.clone()),
          ),
        ),
      },
      lang::POS::Verb { present, past } => Self::Verb {
        present: fmanager.new_from_existing(ComponentTag::Inp1).with_state(
          LineInput::default_or_with((!word.standard).then(|| present.clone())),
        ),
        past: fmanager.new_from_existing(ComponentTag::Inp2).with_state(
          LineInput::default_or_with((!word.standard).then(|| past.clone())),
        ),
      },
      lang::POS::Particle { category } => Self::Particle {
        dropdown: fmanager
          .new_from_existing(ComponentTag::Category)
          .with_state(
            DropdownState::new([
              ParticleCategory::Miscellaneous,
              ParticleCategory::Article,
              ParticleCategory::Comparator,
              ParticleCategory::Conjunction,
              ParticleCategory::Contextual,
              ParticleCategory::Discursive,
              ParticleCategory::Indicator,
              ParticleCategory::Introductor,
            ])
            .with_initial(|o| o == category),
          ),
      },
    }
  }

  fn into_pos(self, key_variant: &str) -> (bool, lang::POS) {
    let mut irregular = false;
    let pos = match self {
      Self::Noun {
        plural,
        definite,
        definite_plural,
      } => lang::POS::Noun {
        plural: expand_generic!(
          irregular,
          plural.into_inner(),
          lang::generate_noun_plural(key_variant)
        ),
        definite: expand_generic!(
          irregular,
          definite.into_inner(),
          lang::generate_noun_definite(key_variant)
        ),
        definite_plural: expand_generic!(
          irregular,
          definite_plural.into_inner(),
          lang::generate_noun_definite_plural(key_variant)
        ),
      },
      Self::Descriptor { plural, adverbial } => lang::POS::Descriptor {
        plural: expand_generic!(
          irregular,
          plural.into_inner(),
          lang::generate_descriptor_plural(key_variant)
        ),
        adverbial: expand_generic!(
          irregular,
          adverbial.into_inner(),
          lang::generate_descriptor_adverbial(key_variant)
        ),
      },
      Self::Verb { present, past } => lang::POS::Verb {
        present: expand_generic!(
          irregular,
          present.into_inner(),
          lang::generate_verb_present(key_variant)
        ),
        past: expand_generic!(
          irregular,
          past.into_inner(),
          lang::generate_verb_past(key_variant)
        ),
      },
      Self::Particle { dropdown } => lang::POS::Particle {
        category: (*dropdown.get()).to_owned(),
      },
    };
    (irregular, pos)
  }

  fn update_placeholders(&mut self, key_variant: &str) {
    match self {
      Self::Noun {
        plural,
        definite,
        definite_plural,
      } => {
        plural.set_placeholder_text(lang::generate_noun_plural(key_variant));
        definite
          .set_placeholder_text(lang::generate_noun_definite(key_variant));
        definite_plural.set_placeholder_text(
          lang::generate_noun_definite_plural(key_variant),
        );
      }
      Self::Descriptor { plural, adverbial } => {
        plural
          .set_placeholder_text(lang::generate_descriptor_plural(key_variant));
        adverbial.set_placeholder_text(lang::generate_descriptor_adverbial(
          key_variant,
        ));
      }
      Self::Verb { present, past } => {
        present.set_placeholder_text(lang::generate_verb_present(key_variant));
        past.set_placeholder_text(lang::generate_verb_past(key_variant));
      }
      _ => {}
    }
  }

  fn handle_ev(
    &mut self,
    fmanager: &FocusManager<ComponentTag>,
    ev: KeyEvent,
  ) -> Option<KeyEvent> {
    match self {
      Self::Noun {
        plural,
        definite,
        definite_plural,
      } => fmanager
        .handle_ev(ev, wedit_evhandlers!(plural, definite, definite_plural)),
      Self::Descriptor { plural, adverbial } => {
        fmanager.handle_ev(ev, wedit_evhandlers!(plural, adverbial))
      }
      Self::Verb { present, past } => {
        fmanager.handle_ev(ev, wedit_evhandlers!(present, past))
      }
      Self::Particle { dropdown } => {
        fmanager.handle_ev(ev, wedit_evhandlers!(dropdown))
      }
    }
  }

  fn render(
    &mut self,
    buf: &mut Buffer,
    pos_area: Rect,
    cat_area: Rect,
    fmanager: &FocusManager<ComponentTag>,
  ) {
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

        let plural_widget = wedit_lineinput!(plural, fmanager, "Plural: ");
        let definite_widget =
          wedit_lineinput!(definite, fmanager, "Definite: ");
        let definite_plural_widget =
          wedit_lineinput!(definite_plural, fmanager, "Def. Plural: ");

        plural_widget.render(plural_line, buf);
        definite_widget.render(definite_line, buf);
        definite_plural_widget.render(definite_plural_line, buf);
      }
      Self::Descriptor { plural, adverbial } => {
        let [plural_line, adverbial_line, _] = Layout::vertical([
          Constraint::Length(1),
          Constraint::Length(1),
          Constraint::Fill(1),
        ])
        .areas(pos_area);

        let plural_widget = wedit_lineinput!(plural, fmanager, "Plural: ");
        let adverbial_widget =
          wedit_lineinput!(adverbial, fmanager, "Adverbial: ");

        plural_widget.render(plural_line, buf);
        adverbial_widget.render(adverbial_line, buf);
      }
      Self::Verb { present, past } => {
        let [present_line, past_line, _] = Layout::vertical([
          Constraint::Length(1),
          Constraint::Length(1),
          Constraint::Fill(1),
        ])
        .areas(pos_area);

        let present_widget = wedit_lineinput!(present, fmanager, "Present: ");
        let past_widget = wedit_lineinput!(past, fmanager, "Past: ");

        present_widget.render(present_line, buf);
        past_widget.render(past_line, buf);
      }
      Self::Particle { dropdown } => {
        let cat_widget = wedit_dropdown!();

        let cat_area = dropdown.limit_area_width(cat_area);
        dropdown
          .widget(cat_widget, fmanager)
          .render(cat_area, buf, dropdown);
      }
    }
  }
}
