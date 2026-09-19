pub struct ROM<'a> {
    pub header: ROMHeader,
    pub buffer: &'a [u8],
}

#[repr(u8)]
#[allow(dead_code)]
#[derive(Debug)]
pub enum ROMSize {
    /// no banking
    BANKS2 = 0,
    BANKS4,
    BANKS8,
    BANKS16,
    BANKS32,
    BANKS64,
    BANKS128,
    /// 4MiB
    BANKS256,
    /// 8MiB
    BANKS512,
}

#[repr(u8)]
#[allow(dead_code)]
#[derive(Debug)]
pub enum RAMSize {
    None = 0,
    Unused,
    /// 8 KiB
    Banks1,
    Banks4,
    Banks16,
    Banks8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code, reason = "used by mapper-aware external RAM access")]
pub enum CartridgeController {
    NoMBC { has_ram: bool },
    MBC1 { has_ram: bool },
    MBC2,
    MBC3 { has_ram: bool, has_timer: bool },
    MBC5 { has_ram: bool, has_rumble: bool },
    Unsupported(u8),
}

#[allow(dead_code, reason = "used by mapper-aware external RAM access")]
impl CartridgeController {
    pub const fn from_cartridge_type(cartridge_type: u8) -> Self {
        match cartridge_type {
            0x00 => Self::NoMBC { has_ram: false },
            0x08 | 0x09 => Self::NoMBC { has_ram: true },
            0x01 => Self::MBC1 { has_ram: false },
            0x02 | 0x03 => Self::MBC1 { has_ram: true },
            0x05 | 0x06 => Self::MBC2,
            0x0f => Self::MBC3 { has_ram: false, has_timer: true },
            0x10 => Self::MBC3 { has_ram: true, has_timer: true },
            0x11 => Self::MBC3 { has_ram: false, has_timer: false },
            0x12 | 0x13 => Self::MBC3 { has_ram: true, has_timer: false },
            0x19 => Self::MBC5 { has_ram: false, has_rumble: false },
            0x1a | 0x1b => Self::MBC5 { has_ram: true, has_rumble: false },
            0x1c => Self::MBC5 { has_ram: false, has_rumble: true },
            0x1d | 0x1e => Self::MBC5 { has_ram: true, has_rumble: true },
            _ => Self::Unsupported(cartridge_type),
        }
    }
}

#[repr(u8)]
#[allow(dead_code)]
pub enum DestinationCode {
    Japan = 0,
    Overseas,
}

// starting at $0100
#[repr(C)]
pub struct ROMHeader {
    entry_point: [u8; 4],
    logo: [u8; 48],
    title: [u8; 16],
    new_licensee_code: [u8; 2],
    sgb_flag: u8,
    cartridge_type: u8,
    pub rom_size: ROMSize,
    pub ram_size: RAMSize,
    destination_code: DestinationCode,
    old_licensee_code: u8,
    mask_rom_version: u8,
    header_checksum: u8,
    global_checksum: [u8; 2],
}

impl Default for ROMHeader {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl ROM<'_> {
    #[allow(dead_code, reason = "used by mapper-aware external RAM access")]
    pub const fn cartridge_controller(&self) -> CartridgeController {
        CartridgeController::from_cartridge_type(self.header.cartridge_type)
    }

    pub fn title(&self) -> &str {
        let mut title_end = 0;
        while self.header.title[title_end] != 0 && title_end < 16 {
            title_end += 1;
        }
        std::str::from_utf8(&self.header.title[..title_end]).expect("invalid utf-8 sequence for ROM title")
    }

    pub fn new(rom_buffer: &[u8]) -> ROM<'_> {
        let mut rom = ROM { header: Default::default(), buffer: rom_buffer };
        let rom_header_size = std::mem::size_of::<ROMHeader>();
        let rom_header_start = 0x0100;
        let rom_header_end = rom_header_start + rom_header_size;
        let rom_header_buffer: &mut [u8; 80] = unsafe { std::mem::transmute(&mut rom.header) };
        rom_header_buffer.copy_from_slice(&rom_buffer[rom_header_start..rom_header_end]);
        rom
    }
}

#[cfg(test)]
mod tests {
    use super::{CartridgeController, ROM};

    #[test]
    fn cartridge_type_classifies_supported_controllers() {
        assert_eq!(
            CartridgeController::from_cartridge_type(0x03),
            CartridgeController::MBC1 { has_ram: true },
        );
        assert_eq!(
            CartridgeController::from_cartridge_type(0x0f),
            CartridgeController::MBC3 { has_ram: false, has_timer: true },
        );
        assert_eq!(
            CartridgeController::from_cartridge_type(0x1e),
            CartridgeController::MBC5 { has_ram: true, has_rumble: true },
        );
        assert_eq!(
            CartridgeController::from_cartridge_type(0xfc),
            CartridgeController::Unsupported(0xfc),
        );
    }

    #[test]
    fn rom_exposes_its_cartridge_controller() {
        let mut buffer = [0; 0x150];
        buffer[0x147] = 0x09;

        let rom = ROM::new(&buffer);

        assert_eq!(rom.cartridge_controller(), CartridgeController::NoMBC { has_ram: true });
    }
}
