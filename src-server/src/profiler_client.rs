use std::{io::{Read, Write}, net::TcpStream};

use anyhow::Context;
use base64::{engine::general_purpose, Engine};
use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::profile_session::ProfileSession;


#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum ProfilerRequestAction {
    ProfilerStart {
        identifier: String,
        flow_name: Option<String>,
    },
    ProfilerEnd {
        save: bool,
    },
    LogMessage {
        timestamp: u64,
        message: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum ProfilerResponse {
    FileResponse {
        identifier: String,
        content: String,
    },
    ErrorResponse {
        identifier: String,
        message: String,
    },
}


pub struct ProfilerClient {
    stream: TcpStream,
    current_session: Option<ProfileSession>,
}

impl ProfilerClient {
    pub fn new(stream: TcpStream) -> ProfilerClient {
        ProfilerClient {
            stream,
            current_session: None,
        }
    }

    pub fn handle_connection(&mut self) {  
        info!("New client connected from: {}", self.stream.peer_addr().unwrap());

        loop {
            if let Err(e) = self.handle_client_request() {
                error!("Error handling client request: {:?}", e);
                break;
            }
        }
    
        info!("Connection closed");
    }
    


    fn handle_client_request(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        // Read the length of the message, the message and then deserialize it
        let mut length_buf = [0u8; 4];
        self.stream.read_exact(&mut length_buf).context("Failed to read message length")?;
        let length = u32::from_be_bytes(length_buf) as usize;
    
        let mut buffer = vec![0; length];
        self.stream.read_exact(&mut buffer).context("Failed to read the complete message")?;
    
        let request: Result<ProfilerRequestAction, serde_json::Error> = serde_json::from_slice(&buffer);

        if let Ok(request) = request {
            // Process the request
            if let Some(response) = self.process_request(request) {
                let response = serde_json::to_vec(&response).unwrap();
                let response_length = response.len() as u32;
                self.stream.write_all(&response_length.to_be_bytes())?;
                self.stream.write_all(&response)?;
            }
        }
    
        Ok(())
    }


    fn process_request(
        &mut self,
        request: ProfilerRequestAction,
    ) -> Option<ProfilerResponse> {
        match request {
            ProfilerRequestAction::ProfilerStart {
                identifier,
                flow_name,
            } => {
                info!("ProfilerStart: {}", identifier);
                self.current_session = Some(ProfileSession::new(identifier, flow_name));
            }
            ProfilerRequestAction::LogMessage {
                timestamp,
                message,
            } => {
                if let Some(session) = self.current_session.as_mut() {
                    session.handle_line(&message, timestamp);
                    // if this resulted in a flow being finished, we should return a response with the flow name and time taken
                }
            }
            ProfilerRequestAction::ProfilerEnd { save } => {
                info!("ProfilerSave: {}", save);
    
                if let Some(session) = self.current_session.take() {
                    if save {
                        match session.create_flamegraph() {
                            Ok(buffer) => {
                                let encoded_content = general_purpose::STANDARD.encode(buffer);
                                // let encoded_content = String::from_utcf8(buffer).unwrap();
                                let response = ProfilerResponse::FileResponse {
                                    identifier: session.identifier,
                                    content: encoded_content,
                                };
    
                                return Some(response);
                            }
                            Err(_) => {
                                error!("Failed to create flamegraph");
                                return Some(ProfilerResponse::ErrorResponse {
                                    identifier: session.identifier,
                                    message: "Failed to create flamegraph".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    
        None
    }
}