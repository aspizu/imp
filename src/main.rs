mod import;
mod parser;
mod transformers;

use std::env::args;
use std::fs::read_to_string;
use std::path::Path;

use parser::*;
use transformers::*;

fn main() {
   let mut args = args();
   args.next();
   let path = args.next().unwrap_or("/dev/stdin".into());
   let path = Path::new(path.as_str());
   let src = read_to_string(path).unwrap();
   let pd = Pd::new(src.as_str());
   let mut ps = Ps::new();
   let mut imports = pd.start(&mut ps).unwrap();
   let required_pd = Pd::new(r#"from __future__ import annotations"#);
   let mut required_ps = Ps::new();
   let required_imports = required_pd.start(&mut required_ps).unwrap();
   imports.extend(required_imports);
   combine_relative_imports(&mut imports);
   separate_absolute_imports(&mut imports);
   imports.sort();
   for i in imports {
      println!("{i}");
   }
   print!("\n\n{}", pd.rest(&mut ps));
}
