use crate::{app::Message, tree::Tree, util::*};
use anyhow::{Context, Error, Result};
use roead::{
    self,
    aamp::{ParamList, Parameter, ParameterIO, ParameterList},
};
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Clone)]
pub struct AIProgram(ParameterIO);

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
        if ["AI", "Action", "Behavior", "Query"]
            .into_iter()
            .any(|k| pio.list(k).is_none())
            || pio.object("DemoAIActionIdx").is_none()
        {
            Err(anyhow::anyhow!("Invalid AI program."))
        } else {
            Ok(Self(pio))
        }
    }

    fn ais(&self) -> Vec<&ParameterList> {
        self.0
            .list("AI")
            .unwrap()
            .lists()
            .iter()
            .map(|(_, v)| v)
            .collect()
    }

    fn actions(&self) -> Vec<&ParameterList> {
        self.0
            .list("Action")
            .unwrap()
            .lists()
            .iter()
            .map(|(_, v)| v)
            .collect()
    }

    fn behaviors(&self) -> Vec<&ParameterList> {
        self.0
            .list("Behavior")
            .unwrap()
            .lists()
            .iter()
            .map(|(_, v)| v)
            .collect()
    }

    fn queries(&self) -> Vec<&ParameterList> {
        self.0
            .list("Query")
            .unwrap()
            .lists()
            .iter()
            .map(|(_, v)| v)
            .collect()
    }

    fn items(&self) -> Vec<&ParameterList> {
        self.0
            .list("AI")
            .unwrap()
            .lists()
            .iter()
            .chain(self.0.list("Action").unwrap().lists().iter())
            .chain(self.0.list("Behavior").unwrap().lists().iter())
            .chain(self.0.list("Query").unwrap().lists().iter())
            .map(|(_, v)| v)
            .collect()
    }

    fn references(&self, idx: usize) -> Result<References> {
        let (children, behaviours): (Vec<_>, Vec<_>) = self
            .ais()
            .into_iter()
            .enumerate()
            .map(
                |(i, ai)| -> Result<(HashMap<usize, u32>, HashMap<usize, u32>)> {
                    let child_refs: HashMap<usize, u32> = ai
                        .object("ChildIdx")
                        .context("Invalid AI")?
                        .params()
                        .iter()
                        .filter(|(_, v)| v.as_int().is_ok() && v.as_int().unwrap() as usize == idx)
                        .map(|(k, _)| (i, *k))
                        .collect();
                    let behaviour_refs: HashMap<usize, u32> = match ai.object("BehaviorIdx") {
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
                .object("DemoAIActionIdx")
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
                    action.object("BehaviorIdx").map(|obj| {
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

    fn ai_to_tree(&self, idx: usize) -> Result<Tree> {
        let items = self.items();
        let ai = items[idx];
        let text = ai
            .object("Def")
            .context("AI missing def")?
            .param("Name")
            .map(|p| p.as_str_ref())
            .or_else(|| {
                ai.object("Def")
                    .unwrap()
                    .param("ClassName")
                    .map(|p| p.as_string32())
            })
            .context("AI missing name or class name")??;
        Ok(Tree(
            JPEN_MAP.get(text).unwrap_or(&text).to_string(),
            idx,
            ai.object("ChildIdx")
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
