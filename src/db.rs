use std::{
  collections::{HashMap, HashSet},
  fmt::Write,
  io::{BufRead, BufReader},
};

use color_eyre::eyre::{self, OptionExt};
use itertools::Itertools;
use rust_fuzzy_search::fuzzy_compare;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::parser::SheetType;

const CURRENT_VERSION_ID: u8 = 1;

#[derive(Serialize, Deserialize)]
pub struct Database {
  version: u8,
  data: HashMap<Uuid, WordData>,
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
  phones: Option<HashMap<String, Vec<Uuid>>>,
  words: Option<HashMap<String, HashSet<Uuid>>>,
  ignore_terminal_Y: bool,
  ignore_H: bool,
}

impl WorkingDatabase {
  fn generate_phones(&mut self) {
    let mut phone_map: HashMap<String, Vec<Uuid>> = HashMap::new();
    for (uuid, word_data) in self.data.iter() {
      let word = &word_data.word;
      for phone in &word.phones {
        let phone = process_phone(&self, phone);
        phone_map
          .entry(phone)
          .or_insert_with(Default::default)
          .push(*uuid);
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
struct WordData {
  word: crate::lang::Word,
}

pub fn update(
  db: &mut WorkingDatabase,
  sheet_ty: SheetType,
  sheet_file: std::path::PathBuf,
) -> eyre::Result<String> {
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
            if let Some(er_less) = key_variant.strip_suffix("er") {
              format!("{er_less}re")
            } else if key_variant.ends_with('e') {
              format!("{key_variant}r")
            } else {
              format!("{key_variant}er")
            }
          } else {
            standard = false;
            proposed
          }
        },
        definite: {
          let proposed = parts.next().ok_or_eyre("malformed sheet")?.to_owned();
          if proposed.is_empty() {
            if let Some(e_less) = key_variant.strip_suffix("e") {
              format!("{e_less}a")
            } else if let Some(er_less) = key_variant.strip_suffix("er") {
              if er_less.ends_with(['t', 'd', 'p', 'b', 'v', 'k', 'g']) {
                format!("{er_less}ret")
              } else {
                format!("{er_less}eret")
              }
            } else {
              format!("{key_variant}et")
            }
          } else {
            standard = false;
            proposed
          }
        },
        definite_plural: {
          let proposed = parts.next().ok_or_eyre("malformed sheet")?.to_owned();
          if proposed.is_empty() {
            if let Some(e_less) = key_variant.strip_suffix("e") {
              format!("{e_less}ene")
            } else if let Some(er_less) = key_variant.strip_suffix("er") {
              if er_less.ends_with(['t', 'd', 'p', 'b', 'v', 'k', 'g']) {
                format!("{er_less}rene")
              } else {
                format!("{er_less}erene")
              }
            } else {
              format!("{key_variant}ene")
            }
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
            if key_variant.ends_with('e') {
              format!("{key_variant}")
            } else {
              format!("{key_variant}e")
            }
          } else {
            standard = false;
            proposed
          }
        },
        adverbial: {
          let proposed = parts.next().ok_or_eyre("malformed sheet")?.to_owned();
          if proposed.is_empty() {
            format!("{key_variant}lig")
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
            format!("{key_variant}r")
          } else {
            standard = false;
            proposed
          }
        },
        past: {
          let proposed = parts.next().ok_or_eyre("malformed sheet")?.to_owned();
          if proposed.is_empty() {
            format!("{key_variant}t")
          } else {
            standard = false;
            proposed
          }
        },
      },
      SheetType::Particles => {
        let category = definition;
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

  Ok(buf)
}

pub fn dump(db: &Database) -> String {
  let mut buf = String::new();
  for (key, value) in db.data.iter() {
    writeln!(buf, "{key:?} : {:?}", value.word).unwrap();
  }
  buf
}

pub fn phones(
  db: &mut WorkingDatabase,
  popularity_low: Option<usize>,
  popularity_high: Option<usize>,
) -> String {
  let mut buf = String::new();

  let phone_map: &HashMap<String, Vec<Uuid>> = match db.phones {
    Some(ref phone_map) => phone_map,
    None => {
      db.generate_phones();
      db.phones.as_ref().unwrap()
    }
  };

  for (phone, related) in phone_map
    .into_iter()
    .filter(|(_, related)| {
      !(popularity_low
        .map(|v| related.len() < v)
        .unwrap_or_default()
        || popularity_high
          .map(|v| related.len() > v)
          .unwrap_or_default())
    })
    .sorted_by_key(|(phone, _)| phone.as_str())
  {
    writeln!(buf, "## {phone} ##").unwrap();
    for word in related
      .iter()
      .map(|uuid| &db.data.get(uuid).unwrap().word.variants[0])
      .sorted()
    {
      write!(buf, "{word}  ").unwrap();
    }
    write!(buf, "\n\n").unwrap();
  }

  buf
}

pub fn phone(db: &mut WorkingDatabase, phone: String) -> String {
  let phone = if db.ignore_terminal_Y {
    phone.strip_suffix("Y").unwrap_or(&phone)
  } else {
    &phone
  };

  let phone_map: &HashMap<String, Vec<Uuid>> = match db.phones {
    Some(ref phone_map) => phone_map,
    None => {
      db.generate_phones();
      db.phones.as_ref().unwrap()
    }
  };

  let mut buf = String::new();

  let Some(related) = phone_map.get(phone) else {
    write!(buf, "phone {phone} not found in any words\n").unwrap();
    return buf;
  };

  write!(buf, "## {phone} ##\n\n").unwrap();

  for word in related
    .iter()
    .map(|uuid| &db.data.get(uuid).unwrap().word.variants[0])
    .sorted()
  {
    write!(buf, "{word}\n").unwrap();
  }

  buf
}

pub fn word(db: &mut WorkingDatabase, word: String, fuzzy: bool) -> String {
  let word_map: &HashMap<String, HashSet<Uuid>> = match db.words {
    Some(ref word_map) => word_map,
    None => {
      db.generate_words();
      db.words.as_ref().unwrap()
    }
  };

  let mut buf = String::new();

  let variants = if fuzzy {
    let mut variants = Vec::new();
    for (_, uuids) in word_map
      .iter()
      .filter(|(proposed, _)| fuzzy_compare(proposed, &word) > 0.5)
    {
      variants.extend(uuids.iter());
    }
    variants
  } else {
    match word_map.get(&word) {
      Some(v) => v,
      None => {
        write!(buf, "could not find word {word} in dictionary").unwrap();
        return buf;
      }
    }
    .iter()
    .collect_vec()
  };

  for word in variants
    .iter()
    .map(|uuid| &db.data.get(uuid).unwrap().word)
    .sorted_by_key(|word| &word.variants[0])
  {
    format_word(db, &mut buf, word);
  }

  buf
}

fn format_word<B: Write>(
  db: &WorkingDatabase,
  buf: &mut B,
  word: &crate::lang::Word,
) {
  write!(buf, "%% {} %%\n", word.variants.iter().join(", ")).unwrap();
  write!(buf, "> {}\n\n", word.pos.ty_str()).unwrap();

  write!(buf, "{}\n\n", word.definition).unwrap();

  write!(
    buf,
    "Phones: {}\n\n",
    word
      .phones
      .iter()
      .map(|phone| process_phone(&db, phone))
      .join(", ")
  )
  .unwrap();

  if !word.standard {
    write!(buf, " * Irregular\n").unwrap();
  }

  const OFFSET: usize = 14;
  match &word.pos {
    crate::lang::POS::Noun {
      plural,
      definite,
      definite_plural,
    } => {
      write!(
        buf,
        "{:<width$} {}\n{:<width$} {}\n{:<width$} {}\n\n",
        "Plural:",
        plural,
        "Definite:",
        definite,
        "Def. Plural:",
        definite_plural,
        width = OFFSET
      )
      .unwrap();
    }
    crate::lang::POS::Descriptor { plural, adverbial } => {
      write!(
        buf,
        "{:<width$} {}\n{:<width$} {}\n\n",
        "Plural:",
        plural,
        "Adverbial:",
        adverbial,
        width = OFFSET
      )
      .unwrap();
    }
    crate::lang::POS::Verb { present, past } => {
      write!(
        buf,
        "{:<width$} {}\n{:<width$} {}\n\n",
        "Present:",
        present,
        "Past:",
        past,
        width = OFFSET
      )
      .unwrap();
    }
    _ => {}
  }

  writeln!(buf).unwrap();
}

pub fn search_english(db: &WorkingDatabase, content: String) -> String {
  let mut buf = String::new();

  for word in db
    .data
    .values()
    .map(|word| &word.word)
    .filter(|word| word.definition.contains(&content))
  {
    format_word(db, &mut buf, word);
  }

  buf
}

#[allow(non_snake_case)]
pub fn set_flags(
  db: &mut WorkingDatabase,
  ignore_terminal_Y: Option<bool>,
  ignore_H: Option<bool>,
) -> String {
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

  buf
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
