use std::ops::{Deref, DerefMut};

use serialport::{self, SerialPort};
use gpio::{GpioOut, GpioValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortControlLines {
    pub dsr: bool,      // Data Set Ready
    pub cts: bool,      // Clear To Send
    pub cd: bool,       // Carrier Detect
    pub ri: bool,       // Ring Indicator
    pub dtr: bool,      // Data Terminal Ready
    pub rts: bool,      // Request To Send
}

impl PortControlLines {
    pub fn new() -> Self {
        PortControlLines {
            dsr: false,
            cts: false,
            cd: false,
            ri: false,
            dtr: false,
            rts: false,
        }
    }
}

/// Trait for serial reflectors ports.
/// 
/// A Serial Reflector ports is a device that reflects  outputs data and control
/// line states from another serial port.
/// 
/// This trait is used to define the behaviour of serial reflector ports, so
/// that they can be used in a generic way, and we can implement capture with
/// interposing serial ports.
/// 
/// This trait allows implementations for define their own wiring for the
/// controls lines, Traditionally, the CTS is connected to to RTS and DSR to DTR,
/// but this is not always the case, and ignores CD and RI. - but in some hardware
/// they could be implemented with for instance gpios.
pub trait SerialReflection {
    /// Sets the port control lines state.
    fn reflect_control_lines(&mut self, lines: &PortControlLines) -> serialport::Result<()>;
    fn capture_control_lines(&mut self) -> serialport::Result<PortControlLines>;
}


pub struct SerialReflectorPort<T>
where T: serialport::SerialPort
{
    port: T,
}

impl<T> SerialReflectorPort<T>
where T: serialport::SerialPort
{
    pub fn new(port: T) -> Self {
        SerialReflectorPort { port }
    }
}

impl<T> Deref for SerialReflectorPort<T>
where T: serialport::SerialPort
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.port
    }
}

impl<T> SerialReflection for SerialReflectorPort<T>
where T: serialport::SerialPort
{
    fn reflect_control_lines(&mut self, lines: &PortControlLines) -> serialport::Result<()>
     {
        self.port.write_data_terminal_ready(lines.dsr)?;
        self.port.write_request_to_send(lines.cts)
    }
    fn capture_control_lines(&mut self) -> serialport::Result<PortControlLines> {
         Ok(PortControlLines{
            dsr: self.port.read_data_set_ready()?,
            cts: self.port.read_clear_to_send()?,
            cd:  self.port.read_carrier_detect()?,
            ri:  self.port.read_ring_indicator()?,
            dtr: false,
            rts: false,            // dtr: port.write_data_terminal_ready(true)?,
            // rts: port.is_request_to_send(),
        })
    }
}

pub trait AdvancedSerialPort : SerialReflection
{
    // Read the RTS Set set state.
    fn read_request_to_send(&mut self) -> serialport::Result<bool> ;
    // Read the DTR Set state.
    fn read_data_terminal_ready(&mut self) -> serialport::Result<bool> ;

    #[inline]
    fn can_set_ring_indicator(&self) -> bool { false } 
    #[inline]
    fn can_set_carrier_detect(&self) -> bool { false }
    #[inline]
    fn can_read_data_terminal_ready(&self) -> bool { false } 
    #[inline]
    fn can_read_request_to_send(&self) -> bool { false }


    /// Sets the ring indicator ouput reflector state.
    #[inline]
    fn set_ring_indicator(&mut self, _level: bool) -> serialport::Result<()> {
        Err(serialport::Error::new(
            serialport::ErrorKind::Unknown,
            "Ring Indicator output not supported",
        ))
    }
    /// Sets the carrier detect output reflector state.
    #[inline]
    fn set_carrier_detect(&mut self, _level: bool) -> serialport::Result<()> {
        Err(serialport::Error::new(
            serialport::ErrorKind::Unknown,
            "Carrier Detect output not supported",
        ))
    }
}

impl<T> SerialReflection for T
where
    T: serialport::SerialPort + AdvancedSerialPort,
{
    fn reflect_control_lines(&mut self, lines: &PortControlLines) -> serialport::Result<()> {
        self.reflect_control_lines(lines)?;  // Do the basic reflection
        if self.can_set_ring_indicator() {
            self.set_ring_indicator(lines.ri)?;
        }
        if self.can_set_carrier_detect() {
            self.set_carrier_detect(lines.cd)?;
        }
        Ok(())
    }
    fn capture_control_lines(&mut self) -> serialport::Result<PortControlLines> {
        Ok(PortControlLines {
            dsr: self.read_data_set_ready().unwrap_or(false),
            cts: self.read_clear_to_send().unwrap_or(false),
            cd: self.read_carrier_detect().unwrap_or(false),
            ri: self.read_ring_indicator().unwrap_or(false),
            dtr: self.read_data_terminal_ready().unwrap_or(false),
            rts: self.read_request_to_send().unwrap_or(false),
        })   
    }   
    
}

pub struct SerialPortWithGpios<T, G>
where
    T: serialport::SerialPort,
    G: GpioOut,
{
    port: T,
    ri_out_gpio: Option<G>,
    cd_out_gpio: Option<G>, 
    last_set_rts: Option<bool>,
    last_set_dtr: Option<bool>,
}

impl<T,G> SerialPortWithGpios<T,G>
where
    T: serialport::SerialPort,
    G: GpioOut,
{
    pub fn new(port: T, ri_out_gpio: Option<G>, cd_out_gpio: Option<G>) -> Self {
        SerialPortWithGpios {
            port,
            ri_out_gpio,
            cd_out_gpio,
            last_set_rts: None,
            last_set_dtr: None,
        }
    }
    pub fn write_request_to_send(&mut self, level: bool) -> serialport::Result<()> {
        self.port.write_request_to_send(level).and_then( |_|{
            self.last_set_rts = Some(level);
            Ok(())
        }).or_else(|e| {
            Err(serialport::Error::new(
                serialport::ErrorKind::Unknown,
                format!("Failed to set Request To Send: {}", e),
            ))
        })
    }
    pub fn write_data_terminal_ready(&mut self, level: bool) -> serialport::Result<()> {
        self.port.write_data_terminal_ready(level).and_then( |_|{
            self.last_set_dtr = Some(level);
            Ok(())
        }).or_else(|e| {
            Err(serialport::Error::new(
                serialport::ErrorKind::Unknown,
                format!("Failed to set Request To Send: {}", e),
            ))
        })
    }
}   

impl<T,G> Deref for SerialPortWithGpios<T,G>
where
    T: serialport::SerialPort,
    G: GpioOut + Send,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.port
    }
}

impl<T,G> DerefMut for SerialPortWithGpios<T,G>
where
    T: serialport::SerialPort,
    G: GpioOut + Send,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.port
    }
}



impl<T,G> SerialReflection for SerialPortWithGpios<T,G>
where
    T: serialport::SerialPort,
    G: GpioOut + Send,
{
    fn reflect_control_lines(&mut self, lines: &PortControlLines) -> serialport::Result<()> {
        //self.port.write_data_set_ready(lines.dsr)?;
        //self.port.write_clear_to_send(lines.cts)?;
        self.set_carrier_detect(lines.cd)?;
        self.set_ring_indicator(lines.ri)?;
        self.write_request_to_send(lines.rts)?;
        self.write_data_terminal_ready(lines.dtr)?;
        Ok(())
    }

    fn capture_control_lines(&mut self) -> serialport::Result<PortControlLines> {
        let mut lines = PortControlLines::new();
        lines.dsr = self.read_data_set_ready()?;
        lines.cts = self.read_clear_to_send()?;
        lines.cd = self.read_carrier_detect()?;
        lines.ri = self.read_ring_indicator()?;
        lines.rts = self.read_request_to_send().unwrap_or(false);
        lines.dtr = self.read_data_terminal_ready().unwrap_or(false);
        Ok(lines)
    }
}



impl<T,G> AdvancedSerialPort for SerialPortWithGpios<T,G>
where
    T: serialport::SerialPort,
    G: GpioOut + Send,
{

    fn can_set_ring_indicator(&self) -> bool {
        self.ri_out_gpio.is_some()
    }
    fn can_set_carrier_detect(&self) -> bool {
        self.cd_out_gpio.is_some()
    }

    

    fn read_request_to_send(&mut self) -> serialport::Result<bool> {
        match self.last_set_rts {
            Some(level) => Ok(level),
            None => {
                Err(serialport::Error::new(
                    serialport::ErrorKind::Unknown,
                    format!("Unknown value in Request To Send")),
                )
            },
        }   
    }   
    fn read_data_terminal_ready(&mut self) -> serialport::Result<bool> {
        match self.last_set_dtr {
            Some(level) => Ok(level),
            None => {
                Err(serialport::Error::new(
                    serialport::ErrorKind::Unknown,
                    format!("Unknown value in Data Terminal Ready")),
                )
            },
        }
    }

    fn set_ring_indicator(&mut self, level: bool) -> serialport::Result<()> {
        if let Some(pin) = self.ri_out_gpio.as_mut() {
            pin.set_value(
                if level {
                    GpioValue::High 
                } else {
                    GpioValue::Low
                }
            ).or_else(|_e| {
                Err(serialport::Error::new(
                    serialport::ErrorKind::Unknown,
                    format!("Failed to set Ring Indicator"),
                ))
            })?;
            Ok(())
        } else {
            return Err(serialport::Error::new(
                serialport::ErrorKind::Unknown,
                "Ring Indicator output not supported",
            ));
        }
    }
    
    fn set_carrier_detect(&mut self, level: bool) -> serialport::Result<()> {
        if let Some(pin) = self.cd_out_gpio.as_mut() {
            pin.set_value(
                if level {
                    GpioValue::High 
                } else {
                    GpioValue::Low
                }
            ).or_else(|_e| {
                Err(serialport::Error::new(
                    serialport::ErrorKind::Unknown,
                    format!("Failed to set Carrier Detect")
                ))
            })?;
            Ok(())
        } else {
            return Err(serialport::Error::new(
                serialport::ErrorKind::Unknown,
                "Carrier Detect output not supported",
            ));
        }
    }
}