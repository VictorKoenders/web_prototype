use framework::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Page, Serialize, Deserialize)]
#[page(path = "/vms", refresh = "5s")]
pub struct Vms {
    #[table]
    #[column(field = "name", header = "Name")]
    #[column(field = "state", header = "State")]
    // #[action(name = "on", action = "turn_on")]
    // #[action(name = "off", action = "turn_off")]
    pub vms: Vec<Vm>,
}

#[async_trait]
impl Constructor for Vms {
    async fn construct(_: Request<()>) -> Result<Self> {
        let mut result = Self { vms: Vec::new() };
        result.load()?;
        Ok(result)
    }
}

impl Vms {
    #[allow(dead_code)]
    pub async fn turn_on(&mut self, _: Request<()>, row: Vm) -> Result {
        std::process::Command::new("virsh")
            .args(&["start", &row.name])
            .stdout(std::process::Stdio::piped())
            .output()?;
        self.load()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn turn_off(&mut self, _: Request<()>, row: Vm) -> Result {
        std::process::Command::new("virsh")
            .args(&["shutdown", &row.name])
            .stdout(std::process::Stdio::piped())
            .output()?;
        self.load()?;
        Ok(())
    }

    #[cfg(not(windows))]
    fn load(&mut self) -> Result {
        let result = std::process::Command::new("virsh")
            .args(&["list", "--all"])
            .stdout(std::process::Stdio::piped())
            .output()?;
        let stdout = String::from_utf8_lossy(&result.stdout);
        *self = Self::parse_str(&stdout)?;
        Ok(())
    }

    #[cfg(windows)]
    fn load(&mut self) -> Result {
        let input = r#" Id   Name           State
-------------------------------
 1    infra          running
 -    trangar-dev    shut off
 -    translucence   shut off
"#;
        *self = Self::parse_str(input)?;
        Ok(())
    }

    fn parse_str(str: &str) -> Result<Self> {
        let regex = regex::Regex::new("\\s{2,}").unwrap();

        let mut result = Self { vms: Vec::new() };
        for (idx, line) in str.lines().skip(2).enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let parts = regex.split(line.trim()).collect::<Vec<_>>();
            if let [_, name, state] = parts.as_slice() {
                result.vms.push(Vm {
                    idx,
                    name: name.trim().to_string(),
                    state: state.trim().to_string(),
                });
            } else {
                return Err(format!("Could not parse line {:?}", line).into());
            }
        }
        Ok(result)
    }
}

#[test]
fn test_vms_parse_str() {
    let input = r#" Id   Name           State
-------------------------------
 1    infra          running
 -    trangar-dev    shut off
 -    translucence   shut off
"#;
    let vms = Vms::parse_str(input).expect("Could not parse input");
    assert_eq!(vms.vms.len(), 3);
    assert_eq!(vms.vms[0].name, "infra");
    assert_eq!(vms.vms[0].state, "running");
    assert_eq!(vms.vms[1].name, "trangar-dev");
    assert_eq!(vms.vms[1].state, "shut off");
    assert_eq!(vms.vms[2].name, "translucence");
    assert_eq!(vms.vms[2].state, "shut off");

    assert!(vms.vms[0].id() != vms.vms[1].id());
    assert!(vms.vms[0].id() != vms.vms[2].id());
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vm {
    idx: usize,
    pub name: String,
    pub state: String,
}

impl TableRow for Vm {
    fn id(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.name.hash(&mut hasher);
        self.state.hash(&mut hasher);
        self.idx.hash(&mut hasher);
        hasher.finish().to_string()
    }
}
