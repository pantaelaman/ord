use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Word {
  pub variants: Vec<String>,
  pub phones: Vec<String>,
  pub definition: String,
  pub standard: bool,
  pub pos: POS,
}

impl Word {
  pub fn is_standard(&self) -> bool {
    !matches!(self.pos, POS::Particle { .. }) && self.standard
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum POS {
  Noun {
    plural: String,
    definite: String,
    definite_plural: String,
  },
  Descriptor {
    plural: String,
    adverbial: String,
  },
  Verb {
    present: String,
    past: String,
  },
  Particle {
    category: ParticleCategory,
  },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum POSType {
  Noun,
  Descriptor,
  Verb,
  Particle,
}

impl PartialEq<POS> for POSType {
  fn eq(&self, other: &POS) -> bool {
    matches!(
      (self, other),
      (POSType::Noun, POS::Noun { .. })
        | (POSType::Descriptor, POS::Descriptor { .. })
        | (POSType::Verb, POS::Verb { .. })
        | (POSType::Particle, POS::Particle { .. })
    )
  }
}

impl POSType {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Noun => "noun",
      Self::Descriptor => "descriptor",
      Self::Verb => "verb",
      Self::Particle => "particle",
    }
  }
}

impl std::fmt::Display for POSType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

impl FromStr for POSType {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(match s {
      "n" | "noun" => Self::Noun,
      "d" | "descriptor" => Self::Descriptor,
      "v" | "verb" => Self::Verb,
      "p" | "particle" => Self::Particle,
      _ => return Err(()),
    })
  }
}

impl POS {
  pub fn ty_str(&self) -> String {
    match self {
      Self::Noun { .. } => String::from("noun"),
      Self::Descriptor { .. } => String::from("descriptor"),
      Self::Verb { .. } => String::from("verb"),
      Self::Particle { category } => format!("particle ({})", category),
    }
  }

  pub fn ty(&self) -> POSType {
    match self {
      Self::Noun { .. } => POSType::Noun,
      Self::Descriptor { .. } => POSType::Descriptor,
      Self::Verb { .. } => POSType::Verb,
      Self::Particle { .. } => POSType::Particle,
    }
  }
}

pub fn generate_noun_plural(key_variant: &str) -> String {
  if let Some(er_less) = key_variant.strip_suffix("er") {
    if er_less.ends_with(['t', 'd', 'p', 'b', 'v', 'k', 'g']) {
      format!("{er_less}re")
    } else {
      format!("{er_less}erer")
    }
  } else if key_variant.ends_with('e') {
    format!("{key_variant}r")
  } else {
    format!("{key_variant}er")
  }
}

pub fn generate_noun_definite(key_variant: &str) -> String {
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
}

pub fn generate_noun_definite_plural(key_variant: &str) -> String {
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
}

pub fn generate_descriptor_plural(key_variant: &str) -> String {
  if key_variant.ends_with('e') {
    format!("{key_variant}")
  } else {
    format!("{key_variant}e")
  }
}

pub fn generate_descriptor_adverbial(key_variant: &str) -> String {
  format!("{key_variant}lig")
}

pub fn generate_verb_present(key_variant: &str) -> String {
  format!("{key_variant}r")
}

pub fn generate_verb_past(key_variant: &str) -> String {
  format!("{key_variant}t")
}

#[derive(
  Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum ParticleCategory {
  #[default]
  Miscellaneous,
  Article,
  Comparator,
  Conjunction,
  Contextual,
  Discursive,
  Indicator,
  Introductor,
}

impl ParticleCategory {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Article => "article",
      Self::Comparator => "comparator",
      Self::Conjunction => "conjunction",
      Self::Contextual => "contextual",
      Self::Discursive => "discursive",
      Self::Indicator => "indicator",
      Self::Introductor => "introductor",
      Self::Miscellaneous => "miscellaneous",
    }
  }
}

impl FromStr for ParticleCategory {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(match s {
      "article" => Self::Article,
      "comparator" => Self::Comparator,
      "conjunction" => Self::Conjunction,
      "contextual" => Self::Contextual,
      "discursive" => Self::Discursive,
      "indicator" => Self::Indicator,
      "introductor" => Self::Introductor,
      "miscellaneous" => Self::Miscellaneous,
      _ => return Err(()),
    })
  }
}

impl std::fmt::Display for ParticleCategory {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}
