use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Word {
  pub variants: Vec<String>,
  pub phones: Vec<String>,
  pub definition: String,
  pub standard: bool,
  pub pos: POS,
}

#[derive(Serialize, Deserialize, Debug)]
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
    category: String,
  },
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
}
