use std::{iter::Peekable, str::FromStr};

use bitflags::bitflags;

use crate::lang::POSType;

#[derive(Clone, Copy, Debug)]
pub enum SheetType {
  Nouns,
  Verbs,
  Descriptors,
  Particles,
}

bitflags! {
  pub struct POSTypeFlags: u8 {
    const NOUN = 0b0001;
    const DESCRIPTOR = 0b0010;
    const VERB = 0b0100;
    const PARTICLE = 0b1000;
    const _ = 0b1111;
  }
}

impl Into<POSTypeFlags> for POSType {
  fn into(self) -> POSTypeFlags {
    match self {
      POSType::Noun => POSTypeFlags::NOUN,
      POSType::Descriptor => POSTypeFlags::DESCRIPTOR,
      POSType::Verb => POSTypeFlags::VERB,
      POSType::Particle => POSTypeFlags::PARTICLE,
    }
  }
}

pub enum Commands {
  Update {
    sheet_ty: SheetType,
    sheet_file: std::path::PathBuf,
  },
  Dump,
  Phone {
    phone_matching: Option<(String, MatchType)>,
    bounds: Option<PopularityBounds>,
  },
  Word {
    word: String,
    match_type: MatchType,
    pos_guards: POSTypeFlags,
    edit: bool,
  },
  Search {
    content: String,
  },
  Choose,
  New,
  #[allow(non_snake_case)]
  Flags {
    ignore_terminal_Y: Option<bool>,
    ignore_H: Option<bool>,
  },
  Quit,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
  #[default]
  Standard,
  Fuzzy,
  Regex,
}

pub enum PopularityBounds {
  Unique,
  LowHigh {
    low: Option<usize>,
    high: Option<usize>,
  },
}

struct ParsingIter<I: Iterator> {
  chrs: Peekable<I>,
  consumed: String,
}

impl<I: Iterator> From<I> for ParsingIter<I> {
  fn from(value: I) -> Self {
    ParsingIter {
      chrs: value.peekable(),
      consumed: String::new(),
    }
  }
}

impl<I: Iterator<Item = char>> Iterator for ParsingIter<I> {
  type Item = char;

  fn next(&mut self) -> Option<Self::Item> {
    let chr = self.chrs.next()?;
    self.consumed.push(chr);
    Some(chr)
  }
}

impl<I: Iterator> ParsingIter<I> {
  fn is_empty(&mut self) -> bool {
    self.chrs.peek().is_none()
  }

  fn get_pos(&self) -> usize {
    self.consumed.len()
  }
}

impl<I: Iterator<Item = char>> ParsingIter<I> {
  fn read_until_space(&mut self) -> (usize, String) {
    let pos = self.consumed.len();
    let mut buf = String::new();
    while let Some(c) = self.chrs.next_if(|c| !c.is_whitespace()) {
      buf.push(c);
      self.consumed.push(c);
    }
    (pos, buf)
  }

  fn skip_space(&mut self) {
    while let Some(c) = self.chrs.next_if(|c| c.is_whitespace()) {
      self.consumed.push(c);
    }
  }

  fn read_flag(&mut self) -> (usize, Option<String>) {
    let pos = self.consumed.len() + 1;
    let res = self.chrs.next_if(|c| *c == '-').map(|_| {
      let (_, out) = self.read_until_space();
      out
    });
    (pos, res)
  }

  fn is_flag(&mut self) -> bool {
    self.chrs.peek().map(|c| *c == '-').unwrap_or_default()
  }

  fn read_onoff(&mut self) -> (usize, Option<bool>) {
    let (pos, val) = self.read_until_space();
    (
      pos,
      match val.as_str() {
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
      },
    )
  }
}

pub struct ParseError {
  pos: usize,
  inp: String,
  error: String,
}

impl std::fmt::Display for ParseError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:\n\n", self.error)?;
    write!(f, "  {}\n", self.inp)?;
    write!(f, "  {:>width$}\n\n", '^', width = self.pos + 1)?;
    Ok(())
  }
}

macro_rules! parse_err {
  ($pos:expr,$inp:ident) => {
    Err(ParseError {
      pos: $pos,
      inp: $inp.to_owned(),
      error: String::from("expected end of command"),
    })
  };
  (
    $pos:expr,$inp:ident,$lit:literal) => {
    Err(ParseError {
      pos: $pos,
      inp: $inp.to_owned(),
      error: String::from($lit),
    })
  };
  ($pos:expr,$inp:ident,$expression:expr) => {
    Err(ParseError {
      pos: $pos,
      inp: $inp.to_owned(),
      error: $expression,
    })
  };
}

pub fn parse(inp: &str) -> Result<Commands, ParseError> {
  let mut parser: ParsingIter<_> = inp.chars().into();
  let (pos, cmd_ty) = parser.read_until_space();
  match cmd_ty.as_str() {
    "dump" => {
      if !parser.is_empty() {
        parse_err!(parser.get_pos(), inp)
      } else {
        Ok(Commands::Dump)
      }
    }
    "phones" => {
      let mut phone_matching: Option<(Option<String>, MatchType)> = None;
      let mut bounds = None;

      loop {
        parser.skip_space();

        if parser.is_empty() {
          break;
        }

        if let (pos, Some(flag)) = parser.read_flag() {
          match flag.as_str() {
            "f" | "fuzzy" => match phone_matching {
              Some((pattern, MatchType::Standard)) => {
                phone_matching = Some((pattern, MatchType::Fuzzy))
              }
              None => phone_matching = Some((None, MatchType::Fuzzy)),
              _ => return parse_err!(pos, inp, "match type already set"),
            },
            "r" | "regex" => match phone_matching {
              Some((pattern, MatchType::Standard)) => {
                phone_matching = Some((pattern, MatchType::Regex))
              }
              None => phone_matching = Some((None, MatchType::Regex)),
              _ => return parse_err!(pos, inp, "match type already set"),
            },
            "u" | "unique" => match bounds {
              Some(PopularityBounds::Unique) => {
                return parse_err!(pos, inp, format!("duplicate flag u"))
              }
              Some(PopularityBounds::LowHigh { .. }) => {
                return parse_err!(
                  pos,
                  inp,
                  "unique bounds are incompatible with popularity bounds"
                )
              }
              None => bounds = Some(PopularityBounds::Unique),
            },
            "l" | "low" => {
              parser.skip_space();
              let (pos, value) = parser.read_until_space();
              let Ok(value) = value.parse::<usize>() else {
                return parse_err!(pos, inp, "expected number");
              };
              match bounds {
                Some(PopularityBounds::Unique) => {
                  return parse_err!(
                    pos,
                    inp,
                    "popularity bounds are incompatible with unique bounds"
                  )
                }
                Some(PopularityBounds::LowHigh { low: Some(_), .. }) => {
                  return parse_err!(
                    pos,
                    inp,
                    "low popularity bound already defined"
                  )
                }
                Some(PopularityBounds::LowHigh { ref mut low, .. }) => {
                  *low = Some(value);
                }
                None => {
                  bounds = Some(PopularityBounds::LowHigh {
                    low: Some(value),
                    high: None,
                  })
                }
              }
            }
            "h" | "high" => {
              parser.skip_space();
              let (pos, value) = parser.read_until_space();
              let Ok(value) = value.parse::<usize>() else {
                return parse_err!(pos, inp, "expected number");
              };
              match bounds {
                Some(PopularityBounds::Unique) => {
                  return parse_err!(
                    pos,
                    inp,
                    "popularity bounds are incompatible with unique bounds"
                  )
                }
                Some(PopularityBounds::LowHigh { high: Some(_), .. }) => {
                  return parse_err!(
                    pos,
                    inp,
                    "high popularity bound already defined"
                  )
                }
                Some(PopularityBounds::LowHigh { ref mut high, .. }) => {
                  *high = Some(value);
                }
                None => {
                  bounds = Some(PopularityBounds::LowHigh {
                    low: Some(value),
                    high: None,
                  })
                }
              }
            }
            f => return parse_err!(pos, inp, format!("unknown flag {f}")),
          }
        } else {
          let (pos, value) = parser.read_until_space();
          phone_matching = match phone_matching {
            Some((Some(_), ..)) => {
              return parse_err!(pos, inp, format!("expected flag"))
            }
            Some((None, match_type)) => Some((Some(value), match_type)),
            None => Some((Some(value), MatchType::Standard)),
          }
        }
      }

      Ok(Commands::Phone {
        phone_matching: match phone_matching {
          Some((Some(phone), match_type)) => Some((phone, match_type)),
          Some((None, ..)) => {
            return parse_err!(parser.get_pos(), inp, "expected phone pattern")
          }
          None => None,
        },
        bounds,
      })
    }
    "choice" => {
      if !parser.is_empty() {
        parse_err!(parser.get_pos(), inp)
      } else {
        Ok(Commands::Choose)
      }
    }
    "search" => {
      parser.skip_space();
      if parser.is_empty() {
        return parse_err!(parser.get_pos(), inp, "expected search terms");
      }
      let content = parser.collect::<String>();
      Ok(Commands::Search { content })
    }
    "flags" => {
      #[allow(non_snake_case)]
      let mut ignore_terminal_Y = None;
      #[allow(non_snake_case)]
      let mut ignore_H = None;
      loop {
        parser.skip_space();
        if parser.is_empty() {
          break;
        }
        let (pos, flag) = parser.read_flag();
        let Some(flag) = flag else {
          return parse_err!(pos, inp, "expected flag");
        };
        match flag.as_str() {
          "Y" => {
            if ignore_terminal_Y.is_some() {
              return parse_err!(pos, inp, "already seen flag Y");
            }
            parser.skip_space();
            let (pos, val) = parser.read_onoff();
            let Some(val) = val else {
              return parse_err!(pos, inp, "expected on/off");
            };
            ignore_terminal_Y = Some(val);
          }
          "H" => {
            if ignore_H.is_some() {
              return parse_err!(pos, inp, "already seen flag H");
            }
            parser.skip_space();
            let (pos, val) = parser.read_onoff();
            let Some(val) = val else {
              return parse_err!(pos, inp, "expected on/off");
            };
            ignore_H = Some(val);
          }
          f => return parse_err!(pos, inp, format!("unknown flag {f}")),
        }
      }
      Ok(Commands::Flags {
        ignore_terminal_Y,
        ignore_H,
      })
    }
    "word" => {
      let mut word = None;
      let mut match_type = MatchType::Standard;
      let mut postyflags = None;
      let mut edit = false;
      loop {
        parser.skip_space();
        if parser.is_empty() {
          break;
        }

        if parser.is_flag() {
          let (pos, Some(flag)) = parser.read_flag() else {
            return parse_err!(pos, inp, "expected flag");
          };
          match flag.as_str() {
            "p" | "pos" => {
              if postyflags.is_some() {
                return parse_err!(pos, inp, "pos guards already set");
              }

              parser.skip_space();
              let (pos, poses) = parser.read_until_space();
              let mut newflags = POSTypeFlags::empty();
              for (pos, chunk) in poses.split(',').scan(pos, |pos, chunk| {
                let stashed = *pos;
                *pos += chunk.len() + 1;
                Some((stashed, chunk))
              }) {
                match POSType::from_str(chunk) {
                  Ok(ty) => newflags |= ty.into(),
                  Err(_) => {
                    return parse_err!(pos, inp, "expected valid POS specifier")
                  }
                }
              }
              postyflags = Some(newflags)
            }
            "f" | "fuzzy" => {
              if match_type != MatchType::Standard {
                return parse_err!(pos, inp, "match type already set");
              } else {
                match_type = MatchType::Fuzzy;
              }
            }
            "r" | "regex" => {
              if match_type != MatchType::Standard {
                return parse_err!(pos, inp, "match type already set");
              } else {
                match_type = MatchType::Regex;
              }
            }
            "e" | "edit" => {
              if edit {
                return parse_err!(pos, inp, "duplicate edit flag");
              } else {
                edit = true;
              }
            }
            f => {
              return parse_err!(pos, inp, format!("unknown flag {f}"));
            }
          }
        } else {
          let (pos, proposed) = parser.read_until_space();
          if !proposed.is_empty() {
            if word.is_some() {
              return parse_err!(
                pos,
                inp,
                format!("unexpected argument {proposed}")
              );
            }
            word = Some(proposed);
          }
        }
      }
      let Some(word) = word else {
        return parse_err!(parser.get_pos(), inp, "expected word argument");
      };
      Ok(Commands::Word {
        word,
        match_type,
        pos_guards: postyflags.unwrap_or(POSTypeFlags::all()),
        edit,
      })
    }
    "new" => {
      if !parser.is_empty() {
        return parse_err!(parser.get_pos(), inp);
      }
      Ok(Commands::New)
    }
    "q" | "quit" => {
      if !parser.is_empty() {
        return parse_err!(parser.get_pos(), inp);
      }
      Ok(Commands::Quit)
    }
    "update" => {
      parser.skip_space();
      let (pos, sheet_ty_raw) = parser.read_until_space();
      let sheet_ty = match sheet_ty_raw.as_str() {
        "n" | "noun" | "nouns" => SheetType::Nouns,
        "v" | "verb" | "verbs" => SheetType::Verbs,
        "d" | "descriptor" | "descriptors" => SheetType::Descriptors,
        "p" | "particle" | "particles" => SheetType::Particles,
        s => {
          return parse_err!(
            pos,
            inp,
            if s.is_empty() {
              String::from("expected sheet type")
            } else {
              format!("unknown sheet type {s}")
            }
          );
        }
      };
      parser.skip_space();
      let (pos, sheet_fpath) = parser.read_until_space();
      if sheet_fpath.is_empty() {
        return parse_err!(pos, inp, "expected sheet filepath");
      }

      if !parser.is_empty() {
        parse_err!(parser.get_pos(), inp)
      } else {
        Ok(Commands::Update {
          sheet_ty,
          sheet_file: sheet_fpath.into(),
        })
      }
    }
    c => parse_err!(pos, inp, format!("unknown command {c}")),
  }
}
