use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use serde::{
    de::Visitor,
    de::{Error, MapAccess},
    Deserialize, Deserializer,
};


#[derive(Debug)]
pub struct ResourceOptions(pub HashMap<String, String>);

impl Default for ResourceOptions {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl Deref for ResourceOptions {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ResourceOptions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ResourceOptions {
    pub fn merge_default(&mut self, base: String) {
        let resources = &mut self.0;
        if !resources.contains_key("base") {
            resources.insert("base".to_string(), base);
        }
    }
}

impl<'de> Deserialize<'de> for ResourceOptions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VVisitor;

        impl<'de> Visitor<'de> for VVisitor {
            type Value = ResourceOptions;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("expected a valid value for ResourceOptions")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let mut options: ResourceOptions = ResourceOptions::default();
                options.insert("base".to_string(), value.to_string());
                Ok(options)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut options: ResourceOptions = ResourceOptions::default();
                while let Some((name, path)) = map.next_entry::<String, String>()? {
                    options.insert(name, path);
                }
                Ok(options)
            }
        }

        deserializer.deserialize_any(VVisitor)
    }
}
