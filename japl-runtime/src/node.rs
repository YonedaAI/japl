/// Node identity and configuration for distributed mode.

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct NodeConfig {
    pub name: String,
    pub listen: Option<String>,
    pub connect: Option<Vec<String>>,
    pub cookie: String,
}

#[allow(dead_code)]
impl NodeConfig {
    pub fn new(name: String) -> Self {
        Self {
            name,
            listen: None,
            connect: None,
            cookie: "japl-default-cookie".to_string(),
        }
    }
}
