use std::iter::Peekable;

#[derive(Clone, Copy, Debug)]
pub enum SheetType {
  Nouns,
  Verbs,
  Descriptors,
  Particles,
}

pub enum Commands {
  Update {
    sheet_ty: SheetType,
    sheet_file: std::path::PathBuf,
  },
  Dump,
  Phones {
    popular_low: Option<usize>,
    popular_high: Option<usize>,
  },
  Phone {
    phone: String,
  },
  Word {
    word: String,
    fuzzy: bool,
  },
  Search {
    content: String,
  },
  New,
  #[allow(non_snake_case)]
  Flags {
    ignore_terminal_Y: Option<bool>,
    ignore_H: Option<bool>,
  },
  Quit,
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

  fn into_consumed(self) -> String {
    self.consumed
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
      if parser.is_empty() {
        Ok(Commands::Phones {
          popular_low: None,
          popular_high: None,
        })
      } else {
        parser.skip_space();
        if !parser.is_flag() {
          let (_, phone) = parser.read_until_space();
          if !parser.is_empty() {
            parse_err!(parser.get_pos(), inp)
          } else {
            Ok(Commands::Phone { phone })
          }
        } else {
          let mut popular_low = None;
          let mut popular_high = None;
          loop {
            let (pos, Some(flag)) = parser.read_flag() else {
              break;
            };
            match flag.as_str() {
              "u" | "unique" => {
                if popular_low.is_some() || popular_high.is_some() {
                  return parse_err!(
                    pos,
                    inp,
                    "unique flag must be used alone"
                  );
                } else {
                  popular_low = Some(1);
                  popular_high = Some(1);
                }
              }
              "p" | "popularity" => {
                if popular_low.is_some() {
                  return parse_err!(
                    pos,
                    inp,
                    "popularity bounds already declared"
                  );
                } else {
                  parser.skip_space();
                  let (pos, num) = parser.read_until_space();
                  if let Ok(v) = num.parse::<usize>() {
                    popular_low = Some(v);
                  } else {
                    return parse_err!(pos, inp, "expected positive number");
                  }
                }
              }
              f => {
                return parse_err!(
                  parser.get_pos(),
                  inp,
                  format!("unknown flag {f}")
                );
              }
            }
            parser.skip_space();
          }
          if !parser.is_empty() {
            parse_err!(parser.get_pos(), inp)
          } else {
            Ok(Commands::Phones {
              popular_low,
              popular_high,
            })
          }
        }
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
      let mut fuzzy = false;
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
            "f" | "fuzzy" => {
              if fuzzy {
                return parse_err!(pos, inp, "duplicate fuzzy flag");
              } else {
                fuzzy = true;
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
      Ok(Commands::Word { word, fuzzy })
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
