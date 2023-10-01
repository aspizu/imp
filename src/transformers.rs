use std::collections::BTreeMap;

use crate::import::*;
use crate::parser::*;

/// Combines relative imports from the same path
/// into a single relative statement.
pub fn combine_relative_imports(imports: &mut Vec<Import>) {
   let mut unique_relative_imports: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
   for (i, import) in imports.iter().enumerate() {
      if let Import::Relative { from, .. } = import {
         if let Some(v) = unique_relative_imports.iter_mut().find_map(|(j, v)| {
            (if let Import::Relative { from: from2, .. } = &imports[*j] { from2 } else { panic!() } == from).then_some(v)
         }) {
            v.push(i);
         } else {
            unique_relative_imports.insert(i, vec![]);
         }
      }
   }
   for (i, v) in unique_relative_imports.iter() {
      let to_combine: Vec<Token> = v
         .iter()
         .flat_map(|j| if let Import::Relative { identifiers, .. } = &imports[*j] { identifiers } else { panic!() })
         .cloned()
         .collect();
      if let Import::Relative { identifiers: modules, .. } = &mut imports[*i] {
         for i in to_combine {
            modules.insert(i);
         }
      } else {
         panic!()
      }
   }
   for (_, v) in unique_relative_imports.iter() {
      let mut i = 0;
      imports.retain(|_| {
         i += 1;
         !v.contains(&(i - 1))
      })
   }
}

/// Separates each absolute import into single absolute imports.
pub fn separate_absolute_imports(imports: &mut Vec<Import>) {
   let mut to_separate = vec![];
   for import in imports.iter_mut() {
      match import {
         Import::Absolute { modules, .. } => {
            let mut done = false;
            modules.retain(|module| {
               if done {
                  to_separate.push(module.clone());
                  false
               } else {
                  done = true;
                  true
               }
            });
         },
         _ => {}
      }
   }
   for module in to_separate {
      imports.push(Import::Absolute { modules: [module].into(), comment: None })
   }
}
