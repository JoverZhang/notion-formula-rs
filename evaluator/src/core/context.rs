use analyzer::analysis::{Property, Ty};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct EvalContext {
    pub properties: Vec<Property>,
    prop_index: HashMap<String, usize>,
}

impl EvalContext {
    pub fn new(properties: Vec<Property>) -> Self {
        let mut prop_index = HashMap::with_capacity(properties.len());
        for (idx, prop) in properties.iter().enumerate() {
            // Keep first occurrence to match analyzer's current linear lookup behavior.
            prop_index.entry(prop.name.clone()).or_insert(idx);
        }
        Self {
            properties,
            prop_index,
        }
    }

    pub fn property(&self, name: &str) -> Option<&Property> {
        self.prop_index
            .get(name)
            .and_then(|idx| self.properties.get(*idx))
    }

    pub fn ty(&self, name: &str) -> Option<&Ty> {
        self.property(name).map(|property| &property.ty)
    }
}
