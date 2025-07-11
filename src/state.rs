use chrono::prelude::*;
use crate::portinfo::PortControlLines;


pub struct SerialEvent {
    pub timestamp: DateTime<Utc>,
    pub data: Vec<u8>,
    pub control_lines: PortControlLines,
}

impl SerialEvent {
    pub fn new(data: Vec<u8>, valid_len: usize, control_lines: PortControlLines) -> Self {
        SerialEvent {
            timestamp: Utc::now(), // Use current time as timestamp
            data: data[..valid_len].to_vec(), // Ensure we only take valid length of data
            control_lines,
        }
    }
    /// Checks if the event contains any data
    pub fn is_insignificant(&self, line_status: &PortControlLines) -> bool {
        self.data.is_empty() && self.control_lines == *line_status
    }
}

