use crate::{app::Category, tree::Tree, util::*};
use anyhow::{Context, Result};
use roead::{
    self,
    aamp::{hash_name, ParamList, Parameter, ParameterIO, ParameterList},
};
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Clone, PartialEq)]
pub struct AIProgram(ParameterIO);

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct References {
    demos: Vec<u32>,
    ai_children: HashMap<usize, u32>,
    ai_behaviours: HashMap<usize, u32>,
    action_behaviours: HashMap<usize, u32>,
}

impl AIProgram {
    pub fn new<P: AsRef<Path>>(file: P) -> Result<Self> {
        let file = file.as_ref();
        let pio = match file.extension() {
            Some(ext) => match ext.to_str().unwrap() {
                "yml" => ParameterIO::from_text(fs::read_to_string(file)?)?,
                _ => ParameterIO::from_binary(fs::read(file)?)?,
            },
            None => ParameterIO::from_binary(fs::read(file)?)?,
        };
        if [
            hash_name("AI"),
            hash_name("Action"),
            hash_name("Behavior"),
            hash_name("Query"),
        ]
        .into_iter()
        .any(|k| pio.lists().get(k).is_none())
            || pio.objects().get(hash_name("DemoAIActionIdx")).is_none()
        {
            Err(anyhow::anyhow!("Invalid AI program."))
        } else {
            Ok(Self(pio))
        }
    }

    pub fn save(&self, file: &Path) -> Result<()> {
        match file.extension() {
            Some(ext) => match ext.to_str().unwrap() {
                "yml" => fs::write(file, self.0.to_text())?,
                _ => fs::write(file, self.0.to_binary())?,
            },
            None => fs::write(file, self.0.to_binary())?,
        };
        Ok(())
    }

    pub fn ais(&self) -> Vec<&ParameterList> {
        self.0
            .lists()
            .get(hash_name("AI"))
            .unwrap()
            .lists()
            .iter()
            .map(|(_, v)| v)
            .collect()
    }

    pub fn actions(&self) -> Vec<&ParameterList> {
        self.0
            .lists()
            .get(hash_name("Action"))
            .unwrap()
            .lists()
            .iter()
            .map(|(_, v)| v)
            .collect()
    }

    pub fn behaviors(&self) -> Vec<&ParameterList> {
        self.0
            .lists()
            .get(hash_name("Behavior"))
            .unwrap()
            .lists()
            .iter()
            .map(|(_, v)| v)
            .collect()
    }

    pub fn queries(&self) -> Vec<&ParameterList> {
        self.0
            .lists()
            .get(hash_name("Query"))
            .unwrap()
            .lists()
            .iter()
            .map(|(_, v)| v)
            .collect()
    }

    pub fn items(&self) -> Vec<&ParameterList> {
        self.0
            .lists()
            .inner()
            .values()
            .flat_map(|cat| cat.lists().inner().values())
            .collect()
    }

    #[allow(dead_code)]
    pub fn items_mut(&mut self) -> Vec<&mut ParameterList> {
        self.0
            .lists_mut()
            .inner_mut()
            .values_mut()
            .flat_map(|cat| cat.lists_mut().inner_mut().values_mut())
            .collect()
    }

    pub fn len(&self) -> usize {
        self.0
            .lists()
            .inner()
            .values()
            .flat_map(|cat| cat.lists().inner().values())
            .count()
    }

    pub fn actions_offset(&self) -> usize {
        self.0.lists().get(hash_name("AI")).unwrap().lists().len()
    }

    pub fn behaviors_offset(&self) -> usize {
        self.0.lists().get(hash_name("AI")).unwrap().lists().len()
            + self
                .0
                .lists()
                .get(hash_name("Action"))
                .unwrap()
                .lists()
                .len()
    }

    pub fn queries_offset(&self) -> usize {
        self.0.lists().get(hash_name("AI")).unwrap().lists().len()
            + self
                .0
                .lists()
                .get(hash_name("Action"))
                .unwrap()
                .lists()
                .len()
            + self
                .0
                .lists()
                .get(hash_name("Behavior"))
                .unwrap()
                .lists()
                .len()
    }

    pub fn item_mut_at_index(&mut self, idx: usize) -> &mut ParameterList {
        let actions_offset = self.actions_offset();
        let behaviors_offset = self.behaviors_offset();
        let queries_offset = self.queries_offset();
        if idx < actions_offset {
            self.0
                .lists_mut()
                .get_mut(hash_name("AI"))
                .unwrap()
                .lists_mut()
                .inner_mut()
                .get_index_mut(idx)
        } else if idx < behaviors_offset {
            self.0
                .lists_mut()
                .get_mut(hash_name("Action"))
                .unwrap()
                .lists_mut()
                .inner_mut()
                .get_index_mut(idx - actions_offset)
        } else if idx < queries_offset {
            self.0
                .lists_mut()
                .get_mut(hash_name("Behavior"))
                .unwrap()
                .lists_mut()
                .inner_mut()
                .get_index_mut(idx - behaviors_offset)
        } else {
            self.0
                .lists_mut()
                .get_mut(hash_name("Query"))
                .unwrap()
                .lists_mut()
                .inner_mut()
                .get_index_mut(idx - queries_offset)
        }
        .map(|(_, v)| v)
        .unwrap()
    }

    pub fn item_at_index(&self, idx: usize) -> &ParameterList {
        if idx < self.actions_offset() {
            self.0
                .lists()
                .get(hash_name("AI"))
                .unwrap()
                .lists()
                .inner()
                .get_index(idx)
        } else if idx < self.behaviors_offset() {
            self.0
                .lists()
                .get(hash_name("Action"))
                .unwrap()
                .lists()
                .inner()
                .get_index(idx - self.actions_offset())
        } else if idx < self.queries_offset() {
            self.0
                .lists()
                .get(hash_name("Behavior"))
                .unwrap()
                .lists()
                .inner()
                .get_index(idx - self.behaviors_offset())
        } else {
            self.0
                .lists()
                .get(hash_name("Query"))
                .unwrap()
                .lists()
                .inner()
                .get_index(idx - self.queries_offset())
        }
        .map(|(_, v)| v)
        .unwrap()
    }

    fn references(&self, idx: usize) -> Result<References> {
        let (children, behaviours): (Vec<_>, Vec<_>) = self
            .ais()
            .into_iter()
            .enumerate()
            .map(
                |(i, ai)| -> Result<(HashMap<usize, u32>, HashMap<usize, u32>)> {
                    let child_refs: HashMap<usize, u32> = ai
                        .objects()
                        .get(hash_name("ChildIdx"))
                        .context("Invalid AI")?
                        .params()
                        .iter()
                        .filter(|(_, v)| v.as_int().is_ok() && v.as_int().unwrap() as usize == idx)
                        .map(|(k, _)| (i, *k))
                        .collect();
                    let behaviour_refs: HashMap<usize, u32> =
                        match ai.objects().get(hash_name("BehaviorIdx")) {
                            Some(obj) => obj
                                .params()
                                .iter()
                                .filter(|(_, v)| {
                                    v.as_int().is_ok() && v.as_int().unwrap() as usize == idx
                                })
                                .map(|(k, _)| (i, *k))
                                .collect(),
                            None => HashMap::default(),
                        };
                    Ok((child_refs, behaviour_refs))
                },
            )
            .collect::<Result<Vec<(HashMap<usize, u32>, HashMap<usize, u32>)>>>()?
            .into_iter()
            .unzip();
        Ok(References {
            demos: self
                .0
                .objects()
                .get(hash_name("DemoAIActionIdx"))
                .unwrap()
                .params()
                .iter()
                .filter(|(_, v)| v.as_int().is_ok() && v.as_int().unwrap() as usize == idx)
                .map(|(k, _)| *k)
                .collect(),
            ai_behaviours: behaviours.into_iter().flatten().collect(),
            ai_children: children.into_iter().flatten().collect(),
            action_behaviours: self
                .actions()
                .into_iter()
                .enumerate()
                .filter_map(|(i, action)| {
                    action.objects().get(hash_name("BehaviorIdx")).map(|obj| {
                        obj.params()
                            .iter()
                            .filter_map(|(key, val)| {
                                if val.as_int().is_ok() && val.as_int().unwrap() as usize == idx {
                                    Some((i, *key))
                                } else {
                                    None
                                }
                            })
                            .collect()
                    })
                })
                .collect::<Vec<HashMap<usize, u32>>>()
                .into_iter()
                .flatten()
                .collect(),
        })
    }

    fn update_indexes(&mut self, old: usize, new: i32) -> Result<()> {
        let refs = self.references(old)?;
        refs.demos.iter().for_each(|key| {
            self.0
                .objects_mut()
                .get_mut(hash_name("DemoAIActionIdx"))
                .unwrap()
                .params_mut()
                .insert(*key, Parameter::Int(new));
        });
        refs.ai_children.iter().for_each(|(idx, key)| {
            self.item_mut_at_index(*idx)
                .objects_mut()
                .get_mut(hash_name("ChildIdx"))
                .unwrap()
                .params_mut()
                .insert(*key, Parameter::Int(new));
        });
        if !refs.ai_behaviours.is_empty() {
            let behaviour_idx = if new > 0 {
                new - (self.behaviors_offset() as i32)
            } else {
                new
            };
            refs.ai_behaviours.iter().for_each(|(idx, key)| {
                self.item_mut_at_index(*idx)
                    .objects_mut()
                    .get_mut(hash_name("BehaviorIdx"))
                    .unwrap()
                    .params_mut()
                    .insert(*key, Parameter::Int(behaviour_idx));
            });
        }
        if !refs.action_behaviours.is_empty() {
            let behaviour_idx = if new > 0 {
                new - (self.behaviors_offset() as i32)
            } else {
                new
            };
            let actions_offset = self.actions_offset();
            refs.action_behaviours.iter().for_each(|(idx, key)| {
                self.item_mut_at_index(actions_offset + idx)
                    .objects_mut()
                    .get_mut(hash_name("BehaviorIdx"))
                    .unwrap()
                    .params_mut()
                    .insert(*key, Parameter::Int(behaviour_idx));
            });
        }
        Ok(())
    }

    pub fn update_names(&mut self, idx: usize, child: String, parent: String) -> Result<()> {
        let item = self.item_mut_at_index(idx);
        let defs = item
            .objects_mut()
            .get_mut(hash_name("Def"))
            .unwrap()
            .params_mut();
        defs.insert(hash_name("Name"), Parameter::StringRef(child.clone()));
        defs.insert(hash_name("GroupName"), Parameter::StringRef(parent));
        let mut child_updates: Vec<(usize, String)> = vec![];
        if let Some(childs) = item.objects_mut().get_mut(hash_name("ChildIdx")) {
            childs
                .params_mut()
                .iter_mut()
                .filter(|(_, v)| **v != Parameter::Int(-1))
                .try_for_each(|(k, v)| -> Result<()> {
                    child_updates.push((v.as_int()? as usize, try_name(*k)));
                    Ok(())
                })?;
        }
        child_updates
            .into_iter()
            .try_for_each(|(i, s)| -> Result<()> {
                self.update_names(i, s, child.clone())?;
                Ok(())
            })?;
        Ok(())
    }

    pub fn add_entry(&mut self, category: Category, class: String) -> Result<usize> {
        let entry = AIDEFS.blank_ai(category, class);
        Ok(match category {
            Category::AI => {
                (self.actions_offset()..self.len())
                    .into_iter()
                    .rev()
                    .try_for_each(|i| -> Result<()> {
                        self.update_indexes(i, i as i32 + 1)?;
                        Ok(())
                    })?;
                let new_idx = self.actions_offset();
                self.0
                    .lists_mut()
                    .get_mut(hash_name("AI"))
                    .unwrap()
                    .lists_mut()
                    .inner_mut()
                    .insert_full(hash_name(&format!("AI_{}", new_idx)), entry)
                    .0
            }
            Category::Action => {
                (self.behaviors_offset()..self.len())
                    .into_iter()
                    .rev()
                    .try_for_each(|i| -> Result<()> {
                        self.update_indexes(i, i as i32 + 1)?;
                        Ok(())
                    })?;
                let new_idx = self.behaviors_offset() - self.actions_offset();
                self.0
                    .lists_mut()
                    .get_mut(hash_name("Action"))
                    .unwrap()
                    .lists_mut()
                    .inner_mut()
                    .insert_full(hash_name(&format!("Action_{}", new_idx)), entry)
                    .0
                    + self.actions_offset()
            }
            Category::Behaviour => {
                (self.queries_offset()..self.len())
                    .into_iter()
                    .rev()
                    .try_for_each(|i| -> Result<()> {
                        self.update_indexes(i, i as i32 + 1)?;
                        Ok(())
                    })?;
                let new_idx = self.queries_offset() - self.behaviors_offset();
                self.0
                    .lists_mut()
                    .get_mut(hash_name("Behavior"))
                    .unwrap()
                    .lists_mut()
                    .inner_mut()
                    .insert_full(hash_name(&format!("Behavior_{}", new_idx)), entry)
                    .0
                    + self.behaviors_offset()
            }
            Category::Query => {
                let new_idx = self.len() - self.queries_offset();
                self.0
                    .lists_mut()
                    .get_mut(hash_name("Query"))
                    .unwrap()
                    .lists_mut()
                    .inner_mut()
                    .insert_full(hash_name(&format!("Query_{}", new_idx)), entry)
                    .0
                    + self.queries_offset()
            }
        })
    }

    pub fn delete_entry(&mut self, idx: usize) -> Result<()> {
        self.update_indexes(idx, -1)?;
        let category = if idx < self.actions_offset() {
            self.0
                .lists_mut()
                .get_mut(hash_name("AI"))
                .unwrap()
                .lists_mut()
                .inner_mut()
                .shift_remove_index(idx);
            "AI"
        } else if idx < self.behaviors_offset() {
            let idx = idx - self.actions_offset();
            self.0
                .lists_mut()
                .get_mut(hash_name("Action"))
                .unwrap()
                .lists_mut()
                .inner_mut()
                .shift_remove_index(idx);
            "Action"
        } else if idx < self.queries_offset() {
            let idx = idx - self.behaviors_offset();
            self.0
                .lists_mut()
                .get_mut(hash_name("Behavior"))
                .unwrap()
                .lists_mut()
                .inner_mut()
                .shift_remove_index(idx);
            "Behavior"
        } else {
            let idx = idx - self.queries_offset();
            self.0
                .lists_mut()
                .get_mut(hash_name("Query"))
                .unwrap()
                .lists_mut()
                .inner_mut()
                .shift_remove_index(idx);
            "Query"
        };
        (idx..self.len())
            .into_iter()
            .try_for_each(|i| -> Result<()> {
                self.update_indexes(i, i as i32 - 1)?;
                Ok(())
            })?;
        let cat = self.0.list_mut(category).unwrap();
        let clone = cat.lists().inner().clone();
        cat.lists_mut().inner_mut().clear();
        cat.lists_mut()
            .inner_mut()
            .extend(clone.into_iter().enumerate().map(|(i, (_, v))| {
                let key = format!("{}_{}", category, i);
                (hash_name(&key), v)
            }));
        Ok(())
    }

    fn roots(&self) -> Result<Vec<usize>> {
        self.ais()
            .into_iter()
            .enumerate()
            .filter_map(|(i, _)| -> Option<Result<usize>> {
                match self.references(i) {
                    Ok(refs) => {
                        if refs.ai_children.is_empty() {
                            Some(Ok(i))
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<Result<Vec<usize>>>()
    }

    pub fn entry_name(ai: &ParameterList) -> Result<&str> {
        Ok(ai
            .objects()
            .get(hash_name("Def"))
            .context("AI missing def")?
            .params()
            .get(&hash_name("Name"))
            .map(|p| p.as_str_ref())
            .or_else(|| {
                ai.objects()
                    .get(hash_name("Def"))
                    .unwrap()
                    .params()
                    .get(&hash_name("ClassName"))
                    .map(|p| p.as_string32())
            })
            .context("AI missing name or class name")?
            .map(|s| JPEN_MAP.get(s).copied().unwrap_or(s))?)
    }

    pub fn entry_name_from_index(&self, idx: usize) -> Result<&str> {
        self.items()
            .get(idx)
            .context("Missing entry index")?
            .objects()
            .get(hash_name("Def"))
            .context("Missing defs")?
            .params()
            .get(&hash_name("ClassName"))
            .context("Missing class name")?
            .as_string()
            .map_err(|e| e.into())
    }

    fn ai_to_tree(&self, idx: usize) -> Result<Tree> {
        let items = self.items();
        let ai = items[idx];
        let text = Self::entry_name(ai)?;
        Ok(Tree(
            JPEN_MAP.get(text).unwrap_or(&text).to_string(),
            idx,
            ai.objects()
                .get(hash_name("ChildIdx"))
                .map(|obj| -> Result<Vec<Tree>> {
                    obj.params()
                        .iter()
                        .filter_map(|(_, v)| {
                            if let Parameter::Int(i) = v {
                                if *i >= 0 {
                                    return Some(self.ai_to_tree(*i as usize));
                                }
                            }
                            None
                        })
                        .collect::<Result<Vec<Tree>>>()
                })
                .unwrap_or_else(|| Ok(vec![]))?,
        ))
    }

    pub fn to_tree(&self) -> Result<Vec<Tree>> {
        self.roots()?
            .into_iter()
            .map(|r| self.ai_to_tree(r))
            .collect()
    }
}
