#[derive(Debug, Clone)]
pub struct UniPath {
    absolute: bool,
    items: Vec<String>,
}

impl UniPath {
    pub fn new(path: &str) -> UniPath {
        let sep = if path.contains('/') { '/' } else { '\\' };

        let absolute = if sep == '/' {
            path.starts_with('/')
        } else {
            path.len() >= 2 && is_drive_letter(&path[..2])
        };

        let items = path
            .split(sep)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        (UniPath {
            absolute,
            items: vec![],
        })
        .join(UniPath {
            absolute: false,
            items,
        })
    }

    pub fn join(&self, path: UniPath) -> UniPath {
        if path.absolute {
            return path;
        }

        let mut items: Vec<String> = vec![];
        for item in path.items {
            if item.is_empty() || item == "." {
                continue;
            } else if item == ".." {
                if items.len() > 0 {
                    if items.last().unwrap() != ".." {
                        items.pop();
                    } else {
                        items.push(item.to_string());
                    }
                } else if self.absolute {
                    items.push(item.to_string());
                }
                continue;
            }
            items.push(item.to_string());
        }

        UniPath {
            absolute: self.absolute,
            items,
        }
    }

    pub fn to_niva_path(&self) -> String {
        let default = "".to_string();
        let first = self.items.first().unwrap_or(&default);

        if self.absolute & !is_drive_letter(first) {
            format!("/{}", self.items.join("/"))
        } else {
            self.items.join("/")
        }
    }

    pub fn to_path_buf(&self) -> std::path::PathBuf {
        let mut path_buf = std::path::PathBuf::new();
        if self.absolute {
            path_buf.push("/");
        }
        for item in &self.items {
            path_buf.push(item);
        }
        path_buf
    }

    pub fn has_upward(&self) ->  bool {
        let first = self.items.first();
        if let Some(first) = first {
            first == ".."
        } else {
            false
        }
    }

    // pub fn remove_upward(&self) -> UniPath { if self.absolute {
    //         self.clone()
    //     } else {
    //         Self {
    //             absolute: self.absolute,
    //             items: self
    //                 .items
    //                 .iter()
    //                 .map(|s| s.to_string())
    //                 .filter(|p| p != "..")
    //                 .collect(),
    //         }
    //     }
    // }
}

pub fn is_drive_letter(item: &str) -> bool {
    if item.len() != 2 {
        return false;
    }

    let bytes = item.as_bytes();
    let first_byte = bytes[0];
    let second_byte = bytes[1];

    (first_byte >= b'A' && first_byte <= b'Z' || first_byte >= b'a' && first_byte <= b'z')
        && second_byte == b':'
}
