use std::{
  collections::{HashMap, HashSet},
  fmt::Write,
  io::{BufRead, BufReader},
};

use either::Either::*;

use color_eyre::eyre::{self, OptionExt};
use itertools::Itertools;
use ratatui::{
  style::{Modifier, Stylize},
  text::{Line, Span, Text},
};
use regex::Regex;
use rust_fuzzy_search::fuzzy_compare;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
  lang::{self, ParticleCategory, Word},
  parser::{MatchType, POSTypeFlags, PopularityBounds, SheetType},
};

const CURRENT_VERSION_ID: u8 = 1;

#[derive(Serialize, Deserialize)]
pub struct Database {
  version: u8,
  data: HashMap<Uuid, WordData>,
}

impl Database {
  pub fn fetch(&self, uuid: &Uuid) -> Option<&WordData> {
    self.data.get(uuid)
  }

  pub fn debug_get_n(&self, n: usize) -> Vec<Uuid> {
    self.data.keys().take(n).copied().collect_vec()
  }
}

impl Default for Database {
  fn default() -> Self {
    Self {
      version: CURRENT_VERSION_ID,
      data: HashMap::new(),
    }
  }
}

#[allow(non_snake_case)]
pub struct WorkingDatabase {
  db: Database,
  phones: Option<HashMap<String, HashSet<Uuid>>>,
  words: Option<HashMap<String, HashSet<Uuid>>>,
  ignore_terminal_Y: bool,
  ignore_H: bool,
}

impl WorkingDatabase {
  fn generate_phones(&mut self) {
    let mut phone_map: HashMap<String, HashSet<Uuid>> = HashMap::new();
    for (uuid, word_data) in self.data.iter() {
      let word = &word_data.word;
      for phone in &word.phones {
        let phone = process_phone(&self, phone);
        phone_map
          .entry(phone)
          .or_insert_with(Default::default)
          .insert(*uuid);
      }
    }
    self.phones = Some(phone_map);
  }

  fn generate_words(&mut self) {
    let mut word_map: HashMap<String, HashSet<Uuid>> = HashMap::new();
    for (uuid, word_data) in self.data.iter() {
      for variant in word_data.word.variants.iter() {
        if let Some(values) = word_map.get_mut(variant) {
          values.insert(*uuid);
        } else {
          let mut values = HashSet::new();
          values.insert(*uuid);
          word_map.insert(variant.to_owned(), values);
        }
      }
    }
    self.words = Some(word_map);
  }

  pub fn get_word_map(&mut self) -> &HashMap<String, HashSet<Uuid>> {
    match self.words {
      Some(ref word_map) => word_map,
      None => {
        self.generate_words();
        self.words.as_ref().unwrap()
      }
    }
  }

  pub fn get_phone_map(&mut self) -> &HashMap<String, HashSet<Uuid>> {
    match self.phones {
      Some(ref phone_map) => phone_map,
      None => {
        self.generate_phones();
        self.phones.as_ref().unwrap()
      }
    }
  }

  pub fn unwrap_phone_map(&self) -> &HashMap<String, HashSet<Uuid>> {
    self.phones.as_ref().unwrap()
  }

  pub fn find_word(
    &mut self,
    word: &str,
    match_type: MatchType,
    pos_guards: POSTypeFlags,
  ) -> Result<Vec<Uuid>, regex::Error> {
    let matches = seek_matches(self.get_word_map(), word, match_type);
    if pos_guards.is_all() {
      return matches;
    }

    matches.map(|uuids| {
      uuids
        .into_iter()
        .filter(|uuid| {
          pos_guards.contains(self.fetch(uuid).unwrap().word.pos.ty().into())
        })
        .collect_vec()
    })
  }

  pub fn update_word(&mut self, target: Uuid, word: Word) -> bool {
    match self.db.data.get_mut(&target) {
      Some(target) => *target = WordData { word },
      None => return false,
    }

    self.phones = None;
    self.words = None;
    true
  }

  pub fn new_word(&mut self, word: Word) -> Uuid {
    let uuid = Uuid::now_v7();
    self.db.data.insert(uuid, WordData { word });

    self.phones = None;
    self.words = None;

    uuid
  }
}
pub fn find_phone<S: AsRef<str>>(
  phone_map: &HashMap<String, HashSet<Uuid>>,
  pattern: Option<(S, MatchType)>,
  bounds: Option<PopularityBounds>,
) -> Result<Vec<(&String, &HashSet<Uuid>)>, regex::Error> {
  let possibles = if let Some((pattern, match_type)) = pattern {
    match match_type {
      MatchType::Regex => {
        let regex = Regex::new(pattern.as_ref())?;
        Left(Left(
          phone_map
            .iter()
            .filter(move |(proposed, _)| regex.is_match(proposed)),
        ))
      }
      MatchType::Fuzzy => {
        Left(Right(phone_map.iter().filter(move |(proposed, _)| {
          fuzzy_compare(proposed, &pattern.as_ref()) > 0.5
        })))
      }
      MatchType::Standard => {
        Right(Left(phone_map.get_key_value(pattern.as_ref()).into_iter()))
      }
    }
  } else {
    Right(Right(phone_map.iter()))
  };

  Ok(match bounds {
    Some(PopularityBounds::Unique) => possibles
      .filter(|(_, targets)| targets.len() == 1)
      .collect_vec(),
    Some(PopularityBounds::LowHigh { low, high }) => possibles
      .filter(|(_, targets)| {
        let len = targets.len();
        low.is_none_or(|low| len >= low) && high.is_none_or(|high| len <= high)
      })
      .collect_vec(),
    None => possibles.collect_vec(),
  })
}

fn seek_matches<'a, U>(
  map: &'a HashMap<String, U>,
  pattern: &str,
  match_type: MatchType,
) -> Result<Vec<Uuid>, regex::Error>
where
  &'a U: IntoIterator<Item = &'a Uuid> + 'a,
{
  Ok(match match_type {
    MatchType::Regex => {
      let regex = Regex::new(pattern)?;
      let mut variants = Vec::new();
      for (_, uuids) in
        map.iter().filter(|(proposed, _)| regex.is_match(proposed))
      {
        variants.extend(uuids.into_iter());
      }
      variants
    }
    MatchType::Fuzzy => {
      let mut variants = Vec::new();
      for (_, uuids) in map
        .iter()
        .filter(|(proposed, _)| fuzzy_compare(proposed, &pattern) > 0.5)
      {
        variants.extend(uuids.into_iter());
      }
      variants
    }
    MatchType::Standard => match map.get(pattern) {
      Some(v) => v.into_iter().copied().collect_vec(),
      None => Vec::new(),
    },
  })
}

impl std::ops::Deref for WorkingDatabase {
  type Target = Database;
  fn deref(&self) -> &Self::Target {
    &self.db
  }
}

impl From<Database> for WorkingDatabase {
  fn from(value: Database) -> Self {
    Self {
      db: value,
      phones: None,
      words: None,
      ignore_terminal_Y: true,
      ignore_H: false,
    }
  }
}

impl Into<Database> for WorkingDatabase {
  fn into(self) -> Database {
    self.db
  }
}

#[derive(Serialize, Deserialize)]
pub struct WordData {
  pub word: crate::lang::Word,
}

pub fn update<'a>(
  db: &mut WorkingDatabase,
  sheet_ty: SheetType,
  sheet_file: std::path::PathBuf,
) -> eyre::Result<Text<'a>> {
  db.phones = None;

  let mut buf = String::new();

  let file = std::fs::File::open(sheet_file)?;
  let reader = BufReader::new(file);

  for line in reader.lines().skip(1) {
    let line = line?;

    let mut parts = line.split('\t');
    let variants = parts
      .next()
      .ok_or_eyre("malformed sheet")?
      .split(',')
      .map(|p| p.to_owned())
      .collect_vec();

    let phones = parts
      .next()
      .ok_or_eyre("malformed sheet")?
      .split(',')
      .map(|p| p.to_owned())
      .collect_vec();

    let mut definition = parts.next().ok_or_eyre("malformed sheet")?.to_owned();

    let key_variant = &variants.get(0).ok_or_eyre("missing variants")?;
    let mut standard = true;

    let pos = match sheet_ty {
      SheetType::Nouns => crate::lang::POS::Noun {
        plural: {
          let proposed = parts.next().ok_or_eyre("malformed sheet")?.to_owned();
          if proposed.is_empty() {
            lang::generate_noun_plural(key_variant)
          } else {
            standard = false;
            proposed
          }
        },
        definite: {
          let proposed = parts.next().ok_or_eyre("malformed sheet")?.to_owned();
          if proposed.is_empty() {
            lang::generate_noun_definite(key_variant)
          } else {
            standard = false;
            proposed
          }
        },
        definite_plural: {
          let proposed = parts.next().ok_or_eyre("malformed sheet")?.to_owned();
          if proposed.is_empty() {
            lang::generate_noun_definite_plural(key_variant)
          } else {
            standard = false;
            proposed
          }
        },
      },
      SheetType::Descriptors => crate::lang::POS::Descriptor {
        plural: {
          let proposed = parts.next().ok_or_eyre("malformed sheet")?.to_owned();
          if proposed.is_empty() {
            lang::generate_descriptor_plural(key_variant)
          } else {
            standard = false;
            proposed
          }
        },
        adverbial: {
          let proposed = parts.next().ok_or_eyre("malformed sheet")?.to_owned();
          if proposed.is_empty() {
            lang::generate_descriptor_adverbial(key_variant)
          } else {
            standard = false;
            proposed
          }
        },
      },
      SheetType::Verbs => crate::lang::POS::Verb {
        present: {
          let proposed = parts.next().ok_or_eyre("malformed sheet")?.to_owned();
          if proposed.is_empty() {
            lang::generate_verb_present(key_variant)
          } else {
            standard = false;
            proposed
          }
        },
        past: {
          let proposed = parts.next().ok_or_eyre("malformed sheet")?.to_owned();
          if proposed.is_empty() {
            lang::generate_verb_past(key_variant)
          } else {
            standard = false;
            proposed
          }
        },
      },
      SheetType::Particles => {
        let category =
          definition.parse::<ParticleCategory>().map_err(|_| {
            eyre::eyre!("unknown particle category {}", definition)
          })?;
        definition = parts.next().ok_or_eyre("malformed sheet")?.to_owned();
        crate::lang::POS::Particle { category }
      }
    };

    let word = crate::lang::Word {
      variants,
      phones,
      definition,
      standard,
      pos,
    };

    writeln!(buf, "{word:?}").unwrap();

    db.db.data.insert(Uuid::now_v7(), WordData { word });
  }

  Ok(buf.into())
}

pub fn dump<'a>(db: &Database) -> Text<'a> {
  let mut buf = String::new();
  for (key, value) in db.data.iter() {
    writeln!(buf, "{key:?} : {:?}", value.word).unwrap();
  }
  buf.into()
}

pub fn format_word<'a>(
  db: &WorkingDatabase,
  word: &crate::lang::Word,
) -> Text<'a> {
  let mut text = Text::default();

  text.push_line(
    format!("%% {} %%", word.variants.iter().join(","))
      .bold()
      .blue(),
  );
  text.push_line(format!("> {}", word.pos.ty_str()).italic());
  text.push_line(Line::default());

  text.push_line(format!("{}", word.definition));
  text.push_line(Line::default());

  text.push_line(Line::from_iter(
    [Span::raw("Phones: ")].into_iter().chain(
      #[allow(unstable_name_collisions)]
      word
        .phones
        .iter()
        .map(|phone| Span::styled(process_phone(&db, phone), Modifier::BOLD))
        .intersperse(Span::raw(", ")),
    ),
  ));
  text.push_line(Line::default());

  if !word.standard {
    text.push_line(" * Irregular".italic());
    text.push_line(Line::default());
  }

  const OFFSET: usize = 14;
  match &word.pos {
    crate::lang::POS::Noun {
      plural,
      definite,
      definite_plural,
    } => {
      text.push_line(Line::from_iter([
        format!("{:<OFFSET$}", "Plural:").into(),
        plural.clone().bold(),
      ]));
      text.push_line(Line::from_iter([
        format!("{:<OFFSET$}", "Definite:").into(),
        definite.clone().bold(),
      ]));
      text.push_line(Line::from_iter([
        format!("{:<OFFSET$}", "Def. Plural:").into(),
        definite_plural.clone().bold(),
      ]));
    }
    crate::lang::POS::Descriptor { plural, adverbial } => {
      text.push_line(Line::from_iter([
        format!("{:<OFFSET$}", "Plural:").into(),
        plural.clone().bold(),
      ]));
      text.push_line(Line::from_iter([
        format!("{:<OFFSET$}", "Adverbial:").into(),
        adverbial.clone().bold(),
      ]));
    }
    crate::lang::POS::Verb { present, past } => {
      text.push_line(Line::from_iter([
        format!("{:<OFFSET$}", "Present:").into(),
        present.clone().bold(),
      ]));
      text.push_line(Line::from_iter([
        format!("{:<OFFSET$}", "Past:").into(),
        past.clone().bold(),
      ]));
    }
    _ => {}
  }

  text.push_line(Line::default());
  text.push_line(Line::default());
  text
}

pub fn search_english<'a>(db: &WorkingDatabase, content: String) -> Text<'a> {
  let mut text = Text::default();

  for word in db
    .data
    .values()
    .map(|word| &word.word)
    .filter(|word| word.definition.contains(&content))
  {
    text.extend(format_word(db, word).into_iter());
  }

  text
}

#[allow(non_snake_case)]
pub fn set_flags<'a>(
  db: &mut WorkingDatabase,
  ignore_terminal_Y: Option<bool>,
  ignore_H: Option<bool>,
) -> Text<'a> {
  if let Some(v) = ignore_terminal_Y {
    db.ignore_terminal_Y = v;
  }
  if let Some(v) = ignore_H {
    db.ignore_H = v;
  }

  // must reset state in case flags changed anything
  db.words = None;
  db.phones = None;

  let mut buf = String::new();

  const FLAG_NAME_WIDTH: usize = 25;

  write!(buf, "Current flags:\n\n").unwrap();
  write!(
    buf,
    "{:<width$} {}\n",
    "Ignore terminal Y:",
    if db.ignore_terminal_Y { "On" } else { "Off" },
    width = FLAG_NAME_WIDTH
  )
  .unwrap();
  write!(
    buf,
    "{:<width$} {}\n",
    "Ignore H:",
    if db.ignore_H { "On" } else { "Off" },
    width = FLAG_NAME_WIDTH
  )
  .unwrap();

  buf.into()
}

fn process_phone(db: &WorkingDatabase, phone: &str) -> String {
  let phone = if db.ignore_terminal_Y {
    phone.strip_suffix("Y").unwrap_or(phone)
  } else {
    phone
  };
  let phone = if db.ignore_H {
    phone.replace("H", "")
  } else {
    phone.to_owned()
  };

  phone
}
