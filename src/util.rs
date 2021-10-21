use std::collections::BTreeMap;

use lazy_static::lazy_static;
use roead::{
    aamp::{hash_name, ParamList, Parameter, ParameterList, ParameterObject},
    types::Vector3f,
};
use serde::Deserialize;

use crate::app::Category;

static JAP_ENG_MAP_JSON: &str = include_str!("../data/jpen.json");
static AI_DEF_JSON: &str = include_str!("../data/aidef.json");
static HASHES_JSON: &str = include_str!("../data/hashes.json");

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AIDefParamValue {
    Bool(bool),
    Int(i32),
    String(String),
    Float(f32),
    Vec3([f32; 3]),
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AIDefEntry {
    None(String),
    Some(AIDef),
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct AIDefParam {
    pub name: String,
    #[serde(rename = "Type")]
    pub param_type: String,
    pub value: Option<AIDefParamValue>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct AIDef {
    pub map_unit_inst_params: Option<Vec<AIDefParam>>,
    pub static_inst_params: Option<Vec<AIDefParam>>,
    pub childs: Option<BTreeMap<String, Vec<AIDefParam>>>,
    pub calc_timing: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct AIDefs {
    #[serde(rename = "AIs")]
    pub ais: BTreeMap<String, AIDefEntry>,
    pub actions: BTreeMap<String, AIDefEntry>,
    pub behaviors: BTreeMap<String, AIDefEntry>,
    pub querys: BTreeMap<String, AIDefEntry>,
}

impl AIDefs {
    pub fn classes<C: std::borrow::Borrow<Category>>(
        &self,
        category: C,
    ) -> impl Iterator<Item = &String> {
        match *category.borrow() {
            Category::AI => self.ais.keys(),
            Category::Action => self.actions.keys(),
            Category::Behaviour => self.behaviors.keys(),
            Category::Query => self.querys.keys(),
        }
    }

    fn default_parameter(param_type: &str, value: &Option<AIDefParamValue>) -> Parameter {
        if let Some(value) = value {
            match value {
                AIDefParamValue::Bool(b) => Parameter::Bool(*b),
                AIDefParamValue::Float(f) => Parameter::F32(*f),
                AIDefParamValue::Int(i) => Parameter::Int(*i),
                AIDefParamValue::String(s) => Parameter::String32(s.clone()),
                AIDefParamValue::Vec3(v) => Parameter::Vec3(Vector3f {
                    x: v[0],
                    y: v[1],
                    z: v[2],
                }),
            }
        } else {
            match param_type {
                "Bool" => Parameter::Bool(false),
                "Float" => Parameter::F32(0.0),
                "Int" => Parameter::Int(0),
                "String" => Parameter::String32(String::new()),
                "Vec3" => Parameter::Vec3(Vector3f::default()),
                _ => unreachable!(),
            }
        }
    }

    pub fn blank_ai(&self, category: Category, class: String) -> ParameterList {
        let mut ai = ParameterList::new();
        let mut defs = ParameterObject::new();
        if matches!(category, Category::AI | Category::Action) {
            defs.params_mut()
                .insert(hash_name("Name"), Parameter::StringRef("".into()));
            defs.params_mut()
                .insert(hash_name("GroupName"), Parameter::StringRef("".into()));
        }
        defs.params_mut()
            .insert(hash_name("ClassName"), Parameter::String32(class.clone()));
        ai.objects_mut().inner_mut().insert(hash_name("Def"), defs);
        if let AIDefEntry::Some(ai_def) = (match category {
            Category::AI => &self.ais,
            Category::Action => &self.actions,
            Category::Behaviour => &self.behaviors,
            Category::Query => &self.querys,
        })
        .get(&class)
        .unwrap()
        {
            if let Some(childs) = &ai_def.childs {
                let mut children = ParameterObject::new();
                for child in childs.keys() {
                    children
                        .params_mut()
                        .insert(hash_name(child.as_str()), Parameter::Int(-1));
                }
                ai.objects_mut()
                    .inner_mut()
                    .insert(hash_name("ChildIdx"), children);
            }
            if let Some(params) = &ai_def.static_inst_params {
                let mut sinst_params = ParameterObject::new();
                for sinst in params {
                    sinst_params.params_mut().insert(
                        hash_name(&sinst.name),
                        Self::default_parameter(&sinst.param_type, &sinst.value),
                    );
                }
                ai.objects_mut()
                    .inner_mut()
                    .insert(hash_name("SInst"), sinst_params);
            }
        }
        ai
    }
}

lazy_static! {
    pub static ref JPEN_MAP: std::collections::HashMap<&'static str, &'static str> =
        serde_json::from_str(JAP_ENG_MAP_JSON).unwrap();
    pub static ref AIDEFS: AIDefs = serde_json::from_str(AI_DEF_JSON).unwrap();
    pub static ref NAME_TABLE: roead::aamp::names::NameTable = {
        let mut table = roead::aamp::names::NameTable::new(true);
        let hashes: std::collections::HashMap<u32, &str> =
            serde_json::from_str(HASHES_JSON).unwrap();
        hashes.into_iter().for_each(|(_, string)| {
            table.add_name(string);
        });
        table
    };
}

#[cached::proc_macro::cached]
pub fn try_name(key: u32) -> String {
    NAME_TABLE
        .get_name(key)
        .map(|n| n.to_owned())
        .unwrap_or_else(|| try_numbered_name(key))
}

#[cached::proc_macro::cached]
fn try_numbered_name(key: u32) -> String {
    for i in (0..=1000).into_iter().map(|i| i.to_string()) {
        for prefix in &["AI_", "Action_", "Behavior_", "Query_"] {
            let test: String = [*prefix, i.as_str()].join("");
            if crc::crc32::checksum_ieee(test.as_bytes()) == key {
                return test;
            }
        }
    }
    key.to_string()
}
