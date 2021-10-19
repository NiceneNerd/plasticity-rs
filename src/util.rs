use std::collections::BTreeMap;

use lazy_static::lazy_static;
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
    AIDef(AIDef),
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
            Category::Behaviour => self.actions.keys(),
            Category::Query => self.querys.keys(),
        }
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
