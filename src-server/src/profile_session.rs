use inferno::flamegraph::{self, Options};
use std::{collections::HashMap, io::{BufReader, Write}};

use crate::microflow_execution::{Activity, MicroflowExecution};


pub struct ActivityExecutionInfo {
    pub name: String,
    pub finished: bool,
    pub total_time: u64,
}

pub struct ProfileSession {
    execution: HashMap<String, MicroflowExecution>,
    pub identifier: String,
    microflow: Option<String>,
}

impl ProfileSession {
    pub fn new(identifier: String, microflow: Option<String>) -> ProfileSession {
        ProfileSession {
            execution: HashMap::new(),
            identifier,
            microflow,
        }
    }

    pub fn handle_line(&mut self, line: &str, timestamp: u64) {
        if let Some((identifier, command)) = line.split_once(" ") {
            if command.starts_with("Executing activity:") {
                let (_, command) = command.split_at("Executing activity: ".len());

                let activity: Activity = serde_json::from_str(command).unwrap();

                let execution = self
                    .execution
                    .entry(identifier.to_string())
                    .or_insert(MicroflowExecution::new(activity.name.clone()));

                if self.microflow.is_none() || self.microflow.as_ref() == Some(&activity.name) {
                    let name = activity.name.clone();
                    execution.handle_activity(activity, timestamp);

                    // let info = ActivityExecutionInfo {
                    //     name: name,
                    //     finished: execution.finished(),
                    //     total_time: execution.execution_time(),
                    // };
                }
            }
        }
    }

    pub fn create_flamegraph(&self) -> Result<Vec<u8>, anyhow::Error> {
        // Create a cursor to simulate the input for inferno
        let mut buffer = Vec::new();
    
        for (_, ex) in self.execution.iter() {
            if ex.finished() {
                ex.write_results(buffer.by_ref())?;
            }
        }
    
        let reader = BufReader::new(&buffer[..]);
        let mut flamegraph_output = Vec::new();
    
        // Use inferno to generate the flamegraph
        let mut options = Options::default();
        flamegraph::from_reader(&mut options, reader, &mut flamegraph_output)?;
    
        Ok(flamegraph_output)
    }
}