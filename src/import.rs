#![allow(unstable_name_collisions)]

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::{self};
use std::hash::Hash;
use std::str;

use itertools::Itertools;

use crate::parser::*;

pub type ModulePath<'a> = Vec<Token<'a>>;

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Debug)]
pub struct Module<'a> {
   pub path: ModulePath<'a>,
   pub alias: Option<Token<'a>>
}

pub type ModuleList<'a> = BTreeSet<Module<'a>>;
pub type IdentifierList<'a> = BTreeSet<Token<'a>>;

impl<'a> Display for Module<'a> {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
      for i in self.path.iter().map(|token| token.slice).intersperse(b".") {
         write!(f, "{}", str::from_utf8(i).unwrap())?
      }
      if let Some(alias) = &self.alias {
         write!(f, " as {}", str::from_utf8(alias.slice).unwrap())?
      }
      Ok(())
   }
}

#[derive(Eq, Clone, Debug)]
pub enum RelativeModule<'a> {
   Named { level: usize, path: ModulePath<'a> },
   Unnamed { level: usize }
}

impl<'a> RelativeModule<'a> {
   fn is_future(&'a self) -> bool {
      matches!(self, Self::Named { level: 0, path } if path.first().is_some_and(|v| v.slice == "__future__".as_bytes()))
   }
}

impl<'a> Ord for RelativeModule<'a> {
   fn cmp(&self, other: &Self) -> Ordering {
      match self {
         Self::Named { level, path } => match other {
            Self::Named { level: level2, path: path2 } => level.cmp(level2).then(path.cmp(path2)),
            Self::Unnamed { .. } => Ordering::Less
         },
         Self::Unnamed { level } => match other {
            Self::Named { .. } => Ordering::Greater,
            Self::Unnamed { level: level2 } => level.cmp(level2).reverse()
         }
      }
   }
}

impl<'a> PartialOrd for RelativeModule<'a> {
   fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
      Some(self.cmp(other))
   }
}

impl<'a> PartialEq for RelativeModule<'a> {
   fn eq(&self, other: &Self) -> bool {
      match (self, other) {
         (Self::Named { level, path }, Self::Named { level: level2, path: path2 }) => level == level2 && path == path2,
         (Self::Unnamed { level }, Self::Unnamed { level: level2 }) => level == level2,
         _ => false
      }
   }
}

impl<'a> Display for RelativeModule<'a> {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
      match self {
         Self::Named { level, path } => {
            for _ in 0..*level {
               write!(f, ".")?
            }
            for i in path.iter().map(|token| token.slice).intersperse(b".") {
               write!(f, "{}", str::from_utf8(i).unwrap())?
            }
         },
         Self::Unnamed { level } =>
            for _ in 0..*level {
               write!(f, ".")?
            },
      }
      Ok(())
   }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Import<'a> {
   Absolute { modules: ModuleList<'a>, comment: Option<Token<'a>> },
   Relative { from: RelativeModule<'a>, identifiers: IdentifierList<'a>, comment: Option<Token<'a>> },
   Wildcard { from: RelativeModule<'a>, comment: Option<Token<'a>> }
}

impl<'a> Ord for Import<'a> {
   fn cmp(&self, other: &Self) -> Ordering {
      match self {
         Self::Absolute { modules, .. } => match other {
            Self::Absolute { modules: other_modules, .. } => modules.cmp(other_modules),
            Self::Relative { .. } => Ordering::Less,
            Self::Wildcard { .. } => Ordering::Less
         },
         Self::Relative { from, .. } if from.is_future() => Ordering::Less,
         Self::Relative { from, .. } => match other {
            Self::Absolute { .. } => Ordering::Greater,
            Self::Relative { from: from2, .. } => from.cmp(from2),
            Self::Wildcard { .. } => Ordering::Less
         },
         Self::Wildcard { from, .. } => match other {
            Self::Absolute { .. } => Ordering::Greater,
            Self::Relative { .. } => Ordering::Greater,
            Self::Wildcard { from: other_from, .. } => from.cmp(other_from)
         }
      }
   }
}

impl<'a> PartialOrd for Import<'a> {
   fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
      Some(self.cmp(other))
   }
}

impl<'a> Display for Import<'a> {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
      match self {
         Self::Absolute { modules, comment } => {
            write!(f, "import ")?;
            let mut i = modules.iter().peekable();
            while let Some(module) = i.next() {
               write!(f, "{module}")?;
               if i.peek().is_some() {
                  write!(f, ", ")?
               }
            }
            if let Some(comment) = comment {
               write!(f, "  {}", str::from_utf8(comment.slice).unwrap())?;
            }
         },
         Self::Relative { from, identifiers, comment } => {
            write!(f, "from {} import ", from)?;
            let mut i = identifiers.iter().peekable();
            while let Some(identifier) = i.next() {
               write!(f, "{}", str::from_utf8(identifier.slice).unwrap())?;
               if i.peek().is_some() {
                  write!(f, ", ")?
               }
            }
            if let Some(comment) = comment {
               write!(f, "  {}", str::from_utf8(comment.slice).unwrap())?;
            }
         },
         Self::Wildcard { from, comment } => {
            write!(f, "from {} import *", from)?;
            if let Some(comment) = comment {
               write!(f, "  {}", str::from_utf8(comment.slice).unwrap())?;
            }
         }
      }
      Ok(())
   }
}
