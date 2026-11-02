//! Talk to the mouse over HID feature reports. Open, do one job, close.

use crate::proto::{self, Packet};
use hidapi::{HidApi, HidDevice};
use std::{thread::sleep, time::Duration};

pub struct Mouse {
    dev: HidDevice,
}

pub struct Status {
    pub profile: u8,
    pub poll_hz: Option<u32>,
    pub leds: [[u8; 7]; 5],
    /// Raw X code per stage (proto::dpi_from_code maps it to dpi).
    pub dpi: [u8; 5],
    pub side_keys: [[u8; 4]; 8],
}

impl Mouse {
    pub fn open() -> Result<Mouse, String> {
        let api = HidApi::new().map_err(|e| e.to_string())?;
        let info = api
            .device_list()
            .find(|d| d.vendor_id() == proto::VID && d.product_id() == proto::PID && d.usage_page() == proto::USAGE_PAGE)
            .ok_or("M811 not found. Plugged in?")?;
        let dev = info.open_device(&api).map_err(|e| e.to_string())?;
        Ok(Mouse { dev })
    }

    /// Read 8 bytes at addr. Works without config mode.
    pub fn read(&self, addr: u16) -> Result<[u8; 8], String> {
        self.dev.send_feature_report(&proto::read_req(addr)).map_err(|e| e.to_string())?;
        sleep(Duration::from_millis(10));
        let mut buf = [0u8; 16];
        buf[0] = 0x02;
        self.dev.get_feature_report(&mut buf).map_err(|e| e.to_string())?;
        Ok(buf[8..16].try_into().unwrap())
    }

    /// Read n bytes starting at any addr (reads are 8-aligned chunks).
    pub fn read_n(&self, addr: u16, n: usize) -> Result<Vec<u8>, String> {
        let lo = addr & !7;
        let mut out = vec![];
        let mut a = lo;
        while out.len() < (addr - lo) as usize + n {
            out.extend(self.read(a)?);
            a += 8;
        }
        Ok(out[(addr - lo) as usize..][..n].to_vec())
    }

    /// Send packets 30 ms apart. If anything fails while in config mode, send exit
    /// anyway, so the mouse never stays frozen.
    pub fn send_all(&self, packets: &[Packet]) -> Result<(), String> {
        let exit = proto::exit();
        let mut entered = false;
        let mut result = Ok(());
        for p in packets {
            if let Err(e) = self.dev.send_feature_report(p) {
                result = Err(format!("send failed: {e}"));
                break;
            }
            if *p == proto::enter() {
                entered = true;
            }
            if *p == exit {
                entered = false;
            }
            sleep(Duration::from_millis(30));
        }
        if entered {
            let _ = self.dev.send_feature_report(&exit);
            sleep(Duration::from_millis(30));
        }
        result
    }

    pub fn unstick(&self) -> Result<(), String> {
        self.dev.send_feature_report(&proto::exit()).map_err(|e| e.to_string())
    }

    pub fn leds(&self) -> Result<[[u8; 7]; 5], String> {
        let mut out = [[0u8; 7]; 5];
        for (i, &a) in proto::LED_ADDRS.iter().enumerate() {
            out[i] = self.read(a - 1)?[1..8].try_into().unwrap();
        }
        Ok(out)
    }

    /// Active profile 1-5. Errors on anything else rather than guessing.
    pub fn active_profile(&self) -> Result<usize, String> {
        match self.read(0x0028)?[4] {
            p @ 0..=4 => Ok(p as usize + 1),
            p => Err(format!("profile word reads {p:02x}, expected 00-04. Run `m811 profile N` first.")),
        }
    }

    pub fn side_key(&self, side: usize) -> Result<[u8; 4], String> {
        Ok(self.read_n(proto::side_key_addr(side), 4)?.try_into().unwrap())
    }

    /// Refuse to touch a side key that holds the slot 0 macro ("full auto").
    pub fn guard_side_key(&self, side: usize) -> Result<(), String> {
        if proto::is_slot0_macro(&self.side_key(side)?) {
            return Err(format!("Side key {side} holds macro slot 0 (\"full auto\"). Not touching it."));
        }
        Ok(())
    }

    pub fn status(&self) -> Result<Status, String> {
        let head = self.read(0x0028)?;
        let poll = self.read(0x0030)?;
        let keys = self.read_n(proto::SIDE_KEY_BASE, 32)?;
        let mut side_keys = [[0u8; 4]; 8];
        for (i, k) in side_keys.iter_mut().enumerate() {
            k.copy_from_slice(&keys[i * 4..i * 4 + 4]);
        }
        Ok(Status {
            profile: head[4] + 1,
            poll_hz: match poll[2] {
                0x01 => Some(1000),
                0x02 => Some(500),
                _ => None,
            },
            leds: self.leds()?,
            dpi: {
                let d = self.read_n(0x0044, 30)?;
                [d[1], d[7], d[13], d[19], d[25]]
            },
            side_keys,
        })
    }
}
