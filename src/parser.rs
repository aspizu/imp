use std::cmp::Ordering;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::str;

use crate::import::*;

#[derive(Eq, PartialOrd, Clone)]
pub struct Token<'a> {
   pub slice: &'a [u8],
   pub i: usize
}

impl<'a> Ord for Token<'a> {
   fn cmp(&self, other: &Self) -> Ordering {
      self.slice.cmp(other.slice)
   }
}

impl<'a> PartialEq for Token<'a> {
   fn eq(&self, other: &Self) -> bool {
      self.slice == other.slice
   }
}

impl<'a> Hash for Token<'a> {
   fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
      self.slice.hash(state)
   }
}

impl<'a> Debug for Token<'a> {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{:#?}", str::from_utf8(self.slice).unwrap())
   }
}

#[derive(Clone, Debug)]
pub struct Pd<'a> {
   src: &'a [u8]
}

#[derive(Clone, Debug)]
pub struct Ps {
   i: usize,
   rest: usize
}

impl Ps {
   pub fn new() -> Self {
      Self { i: 0, rest: 0 }
   }
}

impl<'b> Pd<'b> {
   pub fn new(src: &'b str) -> Self {
      Self { src: src.as_bytes() }
   }

   fn backtrack<T, F>(&self, s: &mut Ps, f: F) -> Option<T>
   where F: Fn(&mut Ps) -> Option<T> {
      let i = s.i;
      let v = f(s);
      if v.is_none() {
         s.i = i;
      }
      v
   }

   fn string(&self, s: &mut Ps, string: &str) -> bool {
      let string = string.as_bytes();
      let mut i = 0;
      while i < string.len() {
         if self.src.len() <= (s.i + i) || self.src[s.i + i] != string[i] {
            return false;
         }
         i += 1;
      }
      s.i += i;
      true
   }

   fn identifier<'a>(&'a self, s: &mut Ps) -> Option<Token<'a>> {
      let mut i = 0;
      loop {
         if self.src.len() <= (s.i + i) {
            break;
         }
         let c = self.src[s.i + i];
         if !(c == b'_' || c.is_ascii_alphanumeric()) {
            break;
         }
         i += 1;
      }
      if i == 0 {
         return None;
      }
      s.i += i;
      Some(Token { slice: &self.src[(s.i - i)..s.i], i: s.i - i })
   }

   fn comment<'a>(&'a self, s: &mut Ps) -> Option<Token<'a>> {
      if !(self.src.len() > s.i && self.src[s.i] == b'#') {
         return None;
      }
      let start = s.i;
      while self.src.len() > s.i && self.src[s.i] != b'\n' {
         s.i += 1;
      }
      if self.src.len() > s.i {
         s.i += 1;
      }
      Some(Token { slice: &self.src[start..s.i - 1], i: start })
   }

   fn whitespace(&self, s: &mut Ps) {
      while self.src.len() > s.i {
         let c = self.src[s.i];
         if !(c == b' ' || c == b'\n') {
            break;
         }
         s.i += 1;
      }
   }

   fn module_path<'a>(&'a self, s: &mut Ps) -> Option<ModulePath<'a>> {
      self.backtrack(s, |s| {
         let mut path = ModulePath::new();
         while let Some(identifier) = self.identifier(s) {
            self.whitespace(s);
            path.push(identifier);
            if !self.string(s, ".") {
               break;
            }
            self.whitespace(s);
         }
         if path.is_empty() {
            return None;
         }
         Some(path)
      })
   }

   fn module<'a>(&'a self, s: &mut Ps) -> Option<Module<'a>> {
      self.backtrack(s, |s| {
         let path = self.module_path(s)?;
         self.whitespace(s);
         if self.string(s, "as") {
            self.whitespace(s);
            Some(Module { path, alias: Some(self.identifier(s)?) })
         } else {
            Some(Module { path, alias: None })
         }
      })
   }

   fn module_list<'a>(&'a self, s: &mut Ps) -> Option<ModuleList<'a>> {
      self.backtrack(s, |s| {
         let mut modules = ModuleList::new();
         self.string(s, "(");
         loop {
            let module = self.module(s)?;
            self.whitespace(s);
            modules.insert(module);
            if !self.string(s, ",") {
               break;
            }
            self.whitespace(s);
         }
         if self.string(s, ")") {
            self.whitespace(s);
         }
         (!modules.is_empty()).then_some(modules)
      })
   }

   fn identifier_list<'a>(&'a self, s: &mut Ps) -> Option<IdentifierList<'a>> {
      self.backtrack(s, |s| {
         let mut identifiers = IdentifierList::new();
         self.string(s, "(");
         loop {
            let identifier = self.identifier(s)?;
            self.whitespace(s);
            identifiers.insert(identifier);
            if !self.string(s, ",") {
               break;
            }
            self.whitespace(s);
         }
         if self.string(s, ")") {
            self.whitespace(s);
         }
         (!identifiers.is_empty()).then_some(identifiers)
      })
   }

   fn relative_module<'a>(&'a self, s: &mut Ps) -> Option<RelativeModule<'a>> {
      self.backtrack(s, |s| {
         let mut level = 0;
         while self.string(s, ".") {
            level += 1;
         }
         if let Some(path) = self.module_path(s) {
            Some(RelativeModule::Named { level, path })
         } else if level == 0 {
            None
         } else {
            Some(RelativeModule::Unnamed { level })
         }
      })
   }

   fn import<'a>(&'a self, s: &mut Ps) -> Option<Import<'a>> {
      self.backtrack(s, |s| {
         if self.string(s, "import") {
            self.whitespace(s);
            let modules = self.module_list(s)?;
            self.whitespace(s);
            let comment = self.comment(s);
            self.whitespace(s);
            Some(Import::Absolute { modules, comment })
         } else if self.string(s, "from") {
            self.whitespace(s);
            let from = self.relative_module(s)?;
            self.whitespace(s);
            if !self.string(s, "import") {
               return None;
            }
            self.whitespace(s);
            if self.string(s, "*") {
               self.whitespace(s);
               let comment = self.comment(s);
               self.whitespace(s);
               Some(Import::Wildcard { from, comment })
            } else {
               let identifiers = self.identifier_list(s)?;
               self.whitespace(s);
               let comment = self.comment(s);
               self.whitespace(s);
               Some(Import::Relative { from, identifiers, comment })
            }
         } else {
            None
         }
      })
   }

   pub fn start<'a>(&'a self, s: &mut Ps) -> Option<Vec<Import<'a>>> {
      self.whitespace(s);
      let mut imports = vec![];
      while let Some(import) = self.import(s) {
         s.rest = s.i;
         self.whitespace(s);
         imports.push(import);
      }
      Some(imports)
   }

   pub fn rest<'a>(&'a self, s: &mut Ps) -> &'a str {
      str::from_utf8(&self.src[s.rest..self.src.len()]).unwrap()
   }
}
