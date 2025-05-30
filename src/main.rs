#![feature(let_chains)]
#![feature(new_range_api)]
use clap::Parser;
use color_eyre::eyre;

mod db;
mod lang;
mod parser;
mod tui;

#[derive(clap::Parser, Debug)]
struct Args {
  #[arg(long = "db", value_name = "DATABASE")]
  db: Option<std::path::PathBuf>,

  #[arg(long = "dry")]
  dryrun: bool,
}

fn main() -> eyre::Result<()> {
  let args = Args::parse();

  let fpath = args.db.unwrap_or_else(|| "ord.db".into());

  let curdb = if std::fs::exists(&fpath)? {
    ciborium::de::from_reader(std::fs::File::open(&fpath)?)?
  } else {
    db::Database::default()
  };

  let findb = tui::run(curdb);
  ratatui::restore();
  let findb = findb?;

  if !args.dryrun {
    let mut fwriter = std::fs::OpenOptions::new()
      .create(true)
      .write(true)
      .open(fpath)?;
    ciborium::ser::into_writer(&findb, &mut fwriter)?;
  }

  Ok(())
}
