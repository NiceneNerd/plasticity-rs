use crate::{tree::Tree, util::*};
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
        let pio = ParameterIO::from_binary(fs::read(file.as_ref())?)?;
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

    pub fn save(&self) -> Vec<u8> {
        self.0.to_binary()
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
            .get(hash_name("AI"))
            .unwrap()
            .lists()
            .iter()
            .chain(
                self.0
                    .lists()
                    .get(hash_name("Action"))
                    .unwrap()
                    .lists()
                    .iter(),
            )
            .chain(
                self.0
                    .lists()
                    .get(hash_name("Behavior"))
                    .unwrap()
                    .lists()
                    .iter(),
            )
            .chain(
                self.0
                    .lists()
                    .get(hash_name("Query"))
                    .unwrap()
                    .lists()
                    .iter(),
            )
            .map(|(_, v)| v)
            .collect()
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

    pub fn entry_name(ai: &'_ ParameterList) -> Result<&'_ str> {
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
        Self::entry_name(self.items().get(idx).context("Out of bounds")?)
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
