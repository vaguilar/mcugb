pub struct ROM<'a> {
    header: ROMHeader,
    rom_buffer: &'a [u8],
}

// starting at $0100
#[repr(C)]
struct ROMHeader {
    entry_point: [u8; 4],
    logo: [u8; 48],
    title: [u8; 16],
    new_licensee_code: [u8; 2],
    sgb_flag: u8,
    cartridge_type: u8,
    rom_size: u8,
    ram_size: u8,
    destination_code: u8,
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
        std::str::from_utf8(&self.header.title).expect("invalid utf-8 sequence for ROM title")
    }

    pub fn new(buffer: &[u8]) -> ROM {
        let mut rom = ROM { header: Default::default(), rom_buffer: buffer };
        let rom_header_size = std::mem::size_of::<ROMHeader>();
        let rom_header_start = 0x0100;
        let rom_header_end = rom_header_start + rom_header_size;
        let rom_header_buffer: &mut [u8; 80] = unsafe { std::mem::transmute(&mut rom.header) };
        rom_header_buffer.copy_from_slice(&buffer[rom_header_start..rom_header_end]);
        rom
    }
}
