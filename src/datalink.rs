use pcap_file::DataLink;
use clap::error::Error;
use std::sync::OnceLock;
use std::collections::HashMap;


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