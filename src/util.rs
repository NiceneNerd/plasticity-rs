use lazy_static::lazy_static;

static JAP_ENG_MAP_JSON: &str = include_str!("../data/jpen.json");
static AI_DEF_JSON: &str = include_str!("../data/aidef.json");
static HASHES_JSON: &str = include_str!("../data/hashes.json");

lazy_static! {
    pub static ref JPEN_MAP: std::collections::HashMap<&'static str, &'static str> =
        serde_json::from_str(JAP_ENG_MAP_JSON).unwrap();
    pub static ref AIDEF_MAP: std::collections::HashMap<&'static str, &'static str> =
        serde_json::from_str(AI_DEF_JSON).unwrap();
    pub static ref NAME_TABLE: roead::aamp::names::NameTable = {
        let mut table = roead::aamp::names::NameTable::new(true);
        let hashes: std::collections::HashMap<u32, &str> =
            serde_json::from_str(HASHES_JSON).unwrap();
        hashes.into_iter().for_each(|(hash, string)| {
            table.add_name(string);
        });
        table
    };
}

#[cached::proc_macro::cached]
fn try_name(key: u32) -> String {
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
