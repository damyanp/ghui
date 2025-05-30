#[derive(Debug)]
pub struct FieldId(pub String);

#[derive(Debug, PartialEq)]
pub struct FieldOptionId(pub String);

#[derive(Debug)]
pub struct Field {
    pub id: FieldId,
    pub name: String,
    pub field_type: FieldType,
    pub options: Vec<(FieldOptionId, String)>,
}

#[derive(Debug)]
pub enum FieldType {
    SingleSelect,
    Iteration,
}

#[derive(Debug)]
pub struct Fields {
    pub project_id: String,
    pub status: Field,
    pub blocked: Field,
    pub epic: Field,
    pub iteration: Field,
}

impl Field {
    pub fn id(&self, name: &str) -> Option<&FieldOptionId> {
        self.options
            .iter()
            .find(|option| option.1 == name)
            .map(|option| &option.0)
    }

    pub fn name(&self, id: &FieldOptionId) -> Option<&str> {
        self.options
            .iter()
            .find(|option| option.0 == *id)
            .map(|option| option.1.as_str())
    }
}
