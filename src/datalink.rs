use chrono::{Timelike, Utc};
use pcap_file::DataLink;
use clap::error::Error;
use std::sync::OnceLock;
use std::collections::HashMap;

use crate::datalink;
use crate::portinfo::PortControlLines;
use crate::state;

const MAX_DATALINK_TYPES: u32 = 512;

/// Builds a map of datalink types.
/// pcap_file::DataLink is a wrapper around the pcap library's datalink types.
/// but there is no way to get the name of the datalink type from the pcap library.
/// This function builds a map of datalink types and their namestring.
/// 
/// The map is built using the integer conversion, and the debug formatting of the datalink type.
pub fn get_datalink_types() -> &'static HashMap<String, DataLink> {
    static DATALINK_TYPES: OnceLock<HashMap<String, DataLink>> = OnceLock::new();
    DATALINK_TYPES.get_or_init(|| -> HashMap<String, DataLink> {
    
        let mut map = HashMap::new();
        
        for i in 0..MAX_DATALINK_TYPES {
            let datalink = DataLink::from(i);
            match datalink {
                DataLink::Unknown(_) => 0,
                _ => {
                    let name = format!("{:?}", datalink);
                    map.insert(name, datalink);
                    1
                }
            };
        
        }
        map
    })
}

/// Parses a datalink type from a string.
/// The string is converted to uppercase and looked up in the map of datalink types.
/// this is used in our clap argument parser.
pub fn parse_datalink(datalink_str:  &str) -> Result<DataLink, Error> {
    let datalink_types = get_datalink_types();
    if let Some(datalink) = datalink_types.get(&datalink_str.to_uppercase()) {
        Ok(datalink.clone())
    } else {
        Err(Error::raw(clap::error::ErrorKind::InvalidValue,format!("Unknown datalink type: {}", datalink_str)))
    }
}


fn raw_encapsulate(data: &[u8]) -> Vec<u8> {
    // This function is the null encapsulation function.
    // It simply returns the data as is.
    //
    // This is used for the RAW and USERx datalink types.
    data.to_vec()
}

fn rtac_encapsulate(new_state: state::SerialEvent, _bus_name: &str) -> Vec<u8> {
    // This function encapsulates the data for the RTAC_SERIAL datalink type.
    // It adds the timestamp, bus name, and port control lines to the data.
    // docs: https://www.tcpdump.org/linktypes/LINKTYPE_RTAC_SERIAL.html

    let mut encapsulated_data = Vec::new();
    let event_type :u8 = 0x01; // Event type for RTAC_SERIAL
    let timestamp = &new_state.timestamp;
    encapsulated_data.extend_from_slice(&timestamp.timestamp().to_le_bytes()[..4]); // 4 bytes for seconds
    encapsulated_data.extend_from_slice(&timestamp.timestamp_micros().to_le_bytes()[..4]); // 4 bytes for microseconds
    encapsulated_data.extend_from_slice(&event_type.to_le_bytes()); // 1 byte for event type
    encapsulated_data.extend_from_slice(&(
        { let mut flags = 0u8;
        if new_state.control_lines.cts { flags |= 0x01; } // CTS
        if new_state.control_lines.cd { flags |= 0x02; } // CD
        if new_state.control_lines.dsr { flags |= 0x04; } // DSR
        if new_state.control_lines.rts { flags |= 0x08; } // RTS
        if new_state.control_lines.dtr { flags |= 0x10 } // DTR
        if new_state.control_lines.ri { flags |= 0x20; } // RI
        flags
    }).to_le_bytes()); // 1 byte for port control lines .
    encapsulated_data.extend_from_slice(&[0x00, 0x00]); //  2 byte footer (reserved for future use)
    encapsulated_data.extend_from_slice(&new_state.data);
    encapsulated_data
}

pub fn get_encapsulated_data(new_state: state::SerialEvent, bus_name: &str, datalink: &DataLink,) -> Result<Vec<u8>, String> {
    match datalink { 
        DataLink::USER0 | DataLink::USER1 | DataLink::USER2 | 
        DataLink::USER3 | DataLink::USER4 | DataLink::USER5 | 
        DataLink::USER6 | DataLink::USER7 | DataLink::USER8 | 
        DataLink::USER9 | DataLink::USER10 | DataLink::USER11 |
        DataLink::USER12 | DataLink::USER13 | DataLink::USER14 |
        DataLink::USER15 | DataLink::RAW => Ok(raw_encapsulate(&new_state.data)),
        DataLink::RTAC_SERIAL => {
            // RTAC_SERIAL is a special case where we encapsulate the data
            // with the timestamp and bus name and port control lines.
            Ok(rtac_encapsulate(new_state, bus_name))
        }
        _ => Err(format!("Unsupported datalink type: {:?}", datalink)),
    }
}
