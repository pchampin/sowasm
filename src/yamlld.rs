//! Utility function for a poor-man's YAML-LD support

pub fn yaml2json<'a>(yaml: &str, json: &'a mut String) -> Result<&'a str, String> {
    let value: serde_json::Value = serde_yaml::from_str(yaml).map_err(|err| format!("{err}"))?;
    let write: &mut Vec<u8> = unsafe {
        // SAFETY: the JSON serializer will generate UTF-8 data
        json.as_mut_vec()
    };
    serde_json::to_writer_pretty(write, &value).map_err(|err| format!("{err}"))?;
    Ok(&json[..])
}

pub fn json2yaml<'a>(json: &str, yaml: &'a mut String) -> Result<&'a str, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|err| format!("{err}"))?;
    let write: &mut Vec<u8> = unsafe {
        // SAFETY: the YAML serializer will generate UTF-8 data
        yaml.as_mut_vec()
    };
    serde_yaml::to_writer(write, &value).map_err(|err| format!("{err}"))?;
    Ok(&yaml[..])
}
