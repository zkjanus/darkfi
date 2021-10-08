use std::collections::{hash_map::Keys, HashMap};

#[derive(Default)]
pub struct Constants {
    pub table: Vec<usize>,
    pub map: HashMap<&'static str, usize>,
}

impl Constants {
    pub fn new() -> Self {
        Constants {
            table: vec![],
            map: HashMap::new(),
        }
    }

    pub fn add(&mut self, variable: &'static str, type_id: usize) {
        let idx = self.table.len();
        self.table.push(type_id);
        self.map.insert(variable, idx);
    }

    pub fn lookup(&self, variable: &str) -> Option<usize> {
        if let Some(idx) = self.map.get(variable) {
            return Some(self.table[*idx]);
        }

        None
    }

    pub fn variables(&self) -> Keys<'_, &str, usize> {
        self.map.keys()
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    pub tokens: Vec<String>,
    pub orig: String,
    pub number: u32,
}

impl Line {
    pub fn new(tokens: Vec<String>, orig: String, number: u32) -> Self {
        Line {
            tokens,
            orig,
            number,
        }
    }
}

// TODO: impl format/display for Line
