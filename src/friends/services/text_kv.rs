use std::collections::HashMap;

pub fn parse_records(text: &str) -> Vec<HashMap<String, String>> {
    let mut records = Vec::new();
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return records;
    }
    for rec in trimmed.split(';').filter(|s| !s.trim().is_empty()) {
        let mut map = HashMap::new();
        for kv in rec.split(',').filter(|s| !s.trim().is_empty()) {
            if let Some((k, v)) = kv.split_once(':') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        records.push(map);
    }
    records
}

pub fn serialize_records(records: &[HashMap<String, String>]) -> String {
    records
        .iter()
        .map(|m| {
            let mut pairs: Vec<String> = m
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect();
            pairs.sort();
            pairs.join(",")
        })
        .collect::<Vec<_>>()
        .join(";")
}

pub fn set_status(
    records: &mut [HashMap<String, String>],
    selector: &dyn Fn(&HashMap<String, String>) -> bool,
    new_status: &str,
) {
    for r in records.iter_mut() {
        if selector(r) {
            r.insert("status".into(), new_status.to_string());
        }
    }
}

pub fn append_record(records: &mut Vec<HashMap<String, String>>, mut new_record: HashMap<String, String>) {
    if !new_record.contains_key("status") {
        new_record.insert("status".into(), "open".into());
    }
    records.push(new_record);
}