use serde_json::Value;

#[derive(Clone, Debug)]
pub enum ScryfallObject {
    Raw(Value),
}

#[derive(Clone, Debug)]
pub struct CardSummary {
    pub name: String,
    pub oracle_text: Option<String>,
    pub mana_cost: Option<String>,
    pub type_line: Option<String>,
    pub colors: Vec<String>,
    pub image_url: Option<String>,
}

impl TryFrom<ScryfallObject> for CardSummary {
    type Error = crate::client::ScryfallError;

    fn try_from(value: ScryfallObject) -> Result<Self, Self::Error> {
        let ScryfallObject::Raw(raw) = value;

        let name = raw
            .get("name")
            .and_then(Value::as_str)
            .ok_or(crate::client::ScryfallError::MissingField("name"))?
            .to_string();

        let oracle_text = raw
            .get("oracle_text")
            .and_then(Value::as_str)
            .map(str::to_string);
        let mana_cost = raw
            .get("mana_cost")
            .and_then(Value::as_str)
            .map(str::to_string);
        let type_line = raw
            .get("type_line")
            .and_then(Value::as_str)
            .map(str::to_string);

        let colors = raw
            .get("colors")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let image_url = raw
            .get("image_uris")
            .and_then(Value::as_object)
            .and_then(|map| map.get("normal").and_then(Value::as_str))
            .map(str::to_string);

        Ok(Self {
            name,
            oracle_text,
            mana_cost,
            type_line,
            colors,
            image_url,
        })
    }
}
