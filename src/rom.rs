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
    pub fn title(&self) -> &str {
        let mut title_end = 0;
        while self.header.title[title_end] != 0 && title_end < 16 {
            title_end += 1;
        }
        std::str::from_utf8(&self.header.title[..title_end]).expect("invalid utf-8 sequence for ROM title")
    }

    pub fn new(rom_buffer: &[u8]) -> ROM {
        let mut rom = ROM { header: Default::default(), buffer: rom_buffer };
        let rom_header_size = std::mem::size_of::<ROMHeader>();
        let rom_header_start = 0x0100;
        let rom_header_end = rom_header_start + rom_header_size;
        let rom_header_buffer: &mut [u8; 80] = unsafe { std::mem::transmute(&mut rom.header) };
        rom_header_buffer.copy_from_slice(&rom_buffer[rom_header_start..rom_header_end]);
        rom
    }
}
