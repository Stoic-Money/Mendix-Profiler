use std::{collections::HashMap, io::Write};

use log::{debug, info};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum ActivityType {
    Start,
    End,
    Break,
    Continue,
    ListLoop,
    #[serde(untagged)]
    Other {
        caption: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Activity {
    pub name: String,
    r#type: String,
    current_activity: ActivityType,
}

struct Flow {
    stack_trace: String,
    timestamp_start: u64,
    total_activity_time: u64,
}

impl Flow {
    fn new(stack_trace: String, timestamp_start: u64) -> Flow {
        Flow {
            stack_trace,
            timestamp_start,
            total_activity_time: 0,
        }
    }
}

pub struct MicroflowExecution {
    pub flow_name: String,
    callstack: Vec<Flow>,
    current_activity: Option<String>,
    prev_timestamp: u64,
    results: HashMap<String, u64>,
    start_time: u64,
    finished: bool,
}

impl MicroflowExecution {
    pub fn new(name: String) -> MicroflowExecution {
        MicroflowExecution {
            flow_name: name,
            callstack: Vec::new(),
            current_activity: None,
            prev_timestamp: 0,
            results: HashMap::new(),
            start_time: 0,
            finished: false,
        }
    }

    pub fn finished(&self) -> bool {
        self.callstack.is_empty() && !self.results.is_empty()
    }

    fn process_activity(&mut self, timestamp: u64) {
        if let Some(current) = self.callstack.last_mut() {
            if let Some(activity_name) = self.current_activity.take() {
                let delta_time = timestamp - self.prev_timestamp;
                debug!(
                    "Activity: {} ::: {} {}",
                    current.stack_trace, activity_name, delta_time
                );

                let key = format!("{};{}", current.stack_trace, activity_name);
                self.add_result(key, delta_time);
            }
        }
        self.prev_timestamp = timestamp;
    }

    pub fn handle_activity(&mut self, command: Activity, timestamp: u64) {
        // if callstack is empty and we have not finished, this activity must be a start activity or this execution is not worth tracking
        // if self.callstack.is_empty() && self.finished {
        //     return;
        // }
        // else if self.callstack.is_empty() {
        //     self.prev_timestamp = timestamp;
        //     self.start_time = timestamp;
        //     self.callstack.push(Flow::new(command.name.clone(), timestamp));
        //     return;
        // }
        
        self.process_activity(timestamp);

        match command.current_activity {
            ActivityType::Start => {
                let stack_trace = if let Some(current) = self.callstack.last() {
                    format!("{};{}", current.stack_trace, command.name)
                } else {
                    // check if name is the same as flow_name
                    if self.flow_name != command.name {

                        return;
                    }
                    self.start_time = timestamp;
                    command.name.clone()
                };

                debug!("Start flow: {}", stack_trace);
                self.callstack.push(Flow::new(stack_trace, timestamp));
            }
            ActivityType::End | ActivityType::Break | ActivityType::Continue => {
                if let Some(popped) = self.callstack.pop() {
                    let delta_time = timestamp - popped.timestamp_start;
                    debug!("End flow: {}, dt: {}", popped.stack_trace, delta_time);

                    self.add_result(popped.stack_trace, delta_time - popped.total_activity_time);

                    if let Some(current) = self.callstack.last_mut() {
                        current.total_activity_time += delta_time;
                    }
                }
            }
            ActivityType::Other { mut caption } => {
                if !self.callstack.is_empty() {
                    caption.insert_str(0, "__");
                    self.current_activity = Some(caption.replace(' ', "_"));
                }
            }
            ActivityType::ListLoop => {}
        }
    }

    fn add_result(&mut self, key: String, delta_time: u64) {
        *self.results.entry(key.clone()).or_default() += delta_time;
    }

    pub fn write_results<W>(&self, mut buffer: W) -> Result<(), anyhow::Error>
    where
        W: Write,
    {
        info!("Writing results for flow: {}", self.flow_name);
        for (trace, time) in self.results.iter() {
            writeln!(buffer, "{} {}", trace, time)?;
        }
        Ok(())
    }

    pub fn execution_time(&self) -> u64 {
        self.prev_timestamp - self.start_time
    }
}
