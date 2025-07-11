use std::{default, ops::{Deref, DerefMut}};

use serialport::{self, SerialPort};
use gpio::{GpioOut, GpioValue};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
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

pub enum AnySerialPort {
    Basic(Box<dyn serialport::SerialPort>),
    Advanced(Box<dyn AdvancedSerialPort>),
}

impl AnySerialPort {
    pub fn as_serial_port(&mut self) -> &mut dyn serialport::SerialPort {
        match self {
            AnySerialPort::Basic(port) => port.as_mut(),
            AnySerialPort::Advanced(port) => port.as_mut()
        }
    }

    pub fn capture_control_lines(&mut self) -> serialport::Result<PortControlLines> {
        match self {
            AnySerialPort::Basic(port) => {
                let mut lines = PortControlLines::new();
                lines.dsr = port.read_data_set_ready()?;
                lines.cts = port.read_clear_to_send()?;
                lines.cd = port.read_carrier_detect()?;
                lines.ri = port.read_ring_indicator()?;
                lines.dtr = false; // DTR is not captured in basic ports
                lines.rts = false; // RTS is not captured in basic ports
                Ok(lines)
            },
            AnySerialPort::Advanced(port) => 
                Ok(PortControlLines {
                    dsr: port.read_data_set_ready().unwrap_or(false),
                    cts: port.read_clear_to_send().unwrap_or(false),
                    cd: port.read_carrier_detect().unwrap_or(false),
                    ri: port.read_ring_indicator().unwrap_or(false),
                    dtr: port.read_data_terminal_ready().unwrap_or(false),
                    rts: port.read_request_to_send().unwrap_or(false),
                    })   
        }
    }
    pub fn reflect_control_lines(&mut self, lines: &PortControlLines) -> serialport::Result<()> {
        match self {
            AnySerialPort::Basic(port) => {
                port.write_data_terminal_ready(lines.dsr)?;
                port.write_request_to_send(lines.cts)
            },
            AnySerialPort::Advanced(port) => {
                port.write_data_terminal_ready(lines.dsr)?;
                port.write_request_to_send(lines.cts)?;
                if port.can_set_ring_indicator() {
                    port.set_ring_indicator(lines.ri)?;
                }
                if port.can_set_carrier_detect() {
                    port.set_carrier_detect(lines.cd)?;
                }
                Ok(())
            } 
        }
    }
}

pub trait AdvancedSerialPort : serialport::SerialPort + Send + Sync {
    // Read the DSR Set set state.
   // fn read_data_set_ready(&mut self) -> serialport::Result<bool>;
    // Read the CTS Set set state.
   // fn read_clear_to_send(&mut self) -> serialport::Result<bool>;
    // Read the CD Set set state.

  //  fn read_carrier_detect(&mut self) -> serialport::Result<bool>;
    // Read the RI Set set state.
   /// fn read_ring_indicator(&mut self) -> serialport::Result<bool>;

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

impl<T,G> std::io::Read for SerialPortWithGpios<T,G>
where
    T: serialport::SerialPort,
    G: GpioOut + Send,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.port.read(buf)
    }
}


impl<T,G> std::io::Write for SerialPortWithGpios<T,G>
where
    T: serialport::SerialPort,
    G: GpioOut + Send,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.port.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.port.flush()
    }
}


impl<T,G> serialport::SerialPort for SerialPortWithGpios<T,G>
where
    T: serialport::SerialPort,
    G: GpioOut + Send,
{
    // Delegate all methods to inner port
    fn name(&self) -> Option<String> { self.port.name() }
    fn baud_rate(&self) -> serialport::Result<u32> { self.port.baud_rate() }
    fn data_bits(&self) -> Result<serialport::DataBits, serialport::Error> { self.port.data_bits() }
    fn flow_control(&self) -> Result<serialport::FlowControl, serialport::Error> { self.port.flow_control() }
    fn parity(&self) -> Result<serialport::Parity, serialport::Error> { self.port.parity() }
    fn stop_bits(&self) -> Result<serialport::StopBits, serialport::Error> { self.port.stop_bits() }
    fn timeout(&self) -> std::time::Duration { self.port.timeout() }
    fn write_request_to_send(&mut self, level: bool) -> serialport::Result<()> { 
        self.port.write_request_to_send(level).and_then(|_| {
                self.last_set_rts = Some(level);
                Ok(())
            }) 
    }
    fn write_data_terminal_ready(&mut self, level: bool) -> serialport::Result<()> { 
        self.port.write_data_terminal_ready(level).and_then(|_| {
            self.last_set_dtr = Some(level);
            Ok(())
        })
    }
    fn read_clear_to_send(&mut self) -> serialport::Result<bool> { self.port.read_clear_to_send() }
    fn read_data_set_ready(&mut self) -> serialport::Result<bool> { self.port.read_data_set_ready() }
    fn read_ring_indicator(&mut self) -> serialport::Result<bool> { self.port.read_ring_indicator() }
    fn read_carrier_detect(&mut self) -> serialport::Result<bool> { self.port.read_carrier_detect() }
    fn bytes_to_read(&self) -> serialport::Result<u32> { self.port.bytes_to_read() }
    fn bytes_to_write(&self) -> serialport::Result<u32> { self.port.bytes_to_write() }
    fn clear(&self, buffer_to_clear: serialport::ClearBuffer) -> serialport::Result<()> { self.port.clear(buffer_to_clear) }
    fn try_clone(&self) -> serialport::Result<Box<dyn serialport::SerialPort>> { self.port.try_clone() }
    fn set_baud_rate(&mut self, baud_rate: u32) -> serialport::Result<()> { self.port.set_baud_rate(baud_rate) }
    fn set_data_bits(&mut self, data_bits: serialport::DataBits) -> serialport::Result<()> { self.port.set_data_bits(data_bits) }
    fn set_flow_control(&mut self, flow_control: serialport::FlowControl) -> serialport::Result<()> { self.port.set_flow_control(flow_control) }
    fn set_parity(&mut self, parity: serialport::Parity) -> serialport::Result<()> { self.port.set_parity(parity) }
    fn set_stop_bits(&mut self, stop_bits: serialport::StopBits) -> serialport::Result<()> { self.port.set_stop_bits(stop_bits) }
    fn set_timeout(&mut self, timeout: std::time::Duration) -> serialport::Result<()> { self.port.set_timeout(timeout) }
    fn set_break(&self) -> serialport::Result<()> { self.port.set_break() }
    fn clear_break(&self) -> serialport::Result<()> { self.port.clear_break() }
 }




impl<T,G> AdvancedSerialPort for SerialPortWithGpios<T,G>
where
    T: serialport::SerialPort + Sync,
    G: GpioOut + Send + Sync,
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