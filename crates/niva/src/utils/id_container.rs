use anyhow::{anyhow, Result};
use std::collections::HashMap;

pub struct IdContainer<Id, Value>
where
    Id: std::ops::Add<Output = Id> + Copy + PartialEq + Eq + std::hash::Hash,
{
    zero: Id,
    one: Id,
    content: HashMap<Id, Value>,
    next_id: Id,
}

impl<Id, Value> IdContainer<Id, Value>
where
    Id: std::ops::Add<Output = Id> + Copy + PartialEq + Eq + std::hash::Hash,
{
    pub fn new(zero: Id, one: Id) -> Self {
        Self {
            zero,
            one,
            content: HashMap::new(),
            next_id: one,
        }
    }

    pub fn get(&self, id: &Id) -> Option<&Value> {
        self.content.get(id)
    }

    pub fn remove(&mut self, id: &Id) -> Option<Value> {
        self.content.remove(id)
    }

    pub fn insert(&mut self, value: Value) -> Result<Id> {
        let next_id = self.next_id()?;
        self.content.insert(next_id, value);
        Ok(next_id)
    }

    pub fn insert_with_id(&mut self, id: Id, value: Value) -> Result<()> {
        self.content.insert(id, value);
        Ok(())
    }

    pub fn next_id(&mut self) -> Result<Id> {
        let mut next_id = self.next_id;
        while next_id != self.zero && self.content.contains_key(&next_id) {
            next_id = next_id + self.one;

            if next_id == self.next_id {
                return Err(anyhow!("Cannot find unused id."));
            }
        }
        self.next_id = next_id + self.one;
        Ok(next_id)
    }
}