#[repr(u8)]
#[allow(dead_code)]
pub enum Joypad {
    Both = 0x00,
    Buttons = 0x10,
    Directional = 0x20,
    None = 0x30,
}

impl From<u8> for Joypad {
    fn from(value: u8) -> Self {
        match value & 0x30 {
            0x00 => Joypad::Both,
            0x10 => Joypad::Buttons,
            0x20 => Joypad::Directional,
            0x30 => Joypad::None,
            _ => unreachable!(),
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Default)]
pub struct LCDControl(u8);

impl LCDControl {
    const BG_AND_WINDOW_ENABLE_PRIORITY: u8 = 1 << 0;
    const OBJ_ENABLE: u8 = 1 << 1;
    const OBJ_DOUBLE_HEIGHT: u8 = 1 << 2;
    const BG_TILE_MAP: u8 = 1 << 3;
    const BG_AND_WINDOW_TILE_DATA_AREA: u8 = 1 << 4;
    const WINDOW_ENABLE: u8 = 1 << 5;
    const WINDOW_TILE_MAP_DATA_AREA: u8 = 1 << 6;
    const LCD_PPU_ENABLE: u8 = 1 << 7;

    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    #[expect(dead_code, reason = "raw register access is useful for diagnostics and serialization")]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[expect(dead_code, reason = "used when DMG background priority behavior is implemented")]
    pub const fn bg_and_window_enable_priority(self) -> bool {
        self.0 & Self::BG_AND_WINDOW_ENABLE_PRIORITY != 0
    }

    pub const fn obj_enable(self) -> bool {
        self.0 & Self::OBJ_ENABLE != 0
    }

    pub const fn obj_double_height(self) -> bool {
        self.0 & Self::OBJ_DOUBLE_HEIGHT != 0
    }

    pub const fn bg_tile_map(self) -> bool {
        self.0 & Self::BG_TILE_MAP != 0
    }

    pub const fn bg_and_window_tile_data_area(self) -> bool {
        self.0 & Self::BG_AND_WINDOW_TILE_DATA_AREA != 0
    }

    #[allow(dead_code, reason = "used by tests and planned window rendering")]
    pub const fn window_enable(self) -> bool {
        self.0 & Self::WINDOW_ENABLE != 0
    }

    #[expect(dead_code, reason = "used when window rendering is implemented")]
    pub const fn window_tile_map_data_area(self) -> bool {
        self.0 & Self::WINDOW_TILE_MAP_DATA_AREA != 0
    }

    #[allow(dead_code, reason = "used by tests and planned LCD enable behavior")]
    pub const fn lcd_ppu_enable(self) -> bool {
        self.0 & Self::LCD_PPU_ENABLE != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PPUMode {
    HBlank = 0,
    VBlank = 1,
    OAMScan = 2,
    Drawing = 3,
}

#[repr(transparent)]
#[derive(Clone, Copy, Default)]
pub struct LCDStatus(u8);

impl LCDStatus {
    const PPU_MODE: u8 = 0b11;

    pub const fn bits(self) -> u8 {
        self.0
    }

    #[allow(dead_code, reason = "used by tests and planned STAT mode ownership")]
    pub fn ppu_mode(self) -> PPUMode {
        match self.0 & Self::PPU_MODE {
            0 => PPUMode::HBlank,
            1 => PPUMode::VBlank,
            2 => PPUMode::OAMScan,
            3 => PPUMode::Drawing,
            _ => unreachable!(),
        }
    }

    pub fn set_ppu_mode(&mut self, mode: PPUMode) {
        self.0 = (self.0 & !Self::PPU_MODE) | mode as u8;
    }

    #[expect(dead_code, reason = "used when LY/LYC coincidence updates are implemented")]
    pub fn set_lyc_equal(&mut self, equal: bool) {
        const LYC_EQUAL: u8 = 1 << 2;
        if equal {
            self.0 |= LYC_EQUAL;
        } else {
            self.0 &= !LYC_EQUAL;
        }
    }
}

/// Memory-mapped registers spanning $FE00-$FFFF.
#[repr(C)]
pub struct IORegisters {
    /// $FE00-$FE9F
    pub sprites: [u8; 160],
    _prohibited: [u8; 96],
    /// $FF00
    pub joypad: u8,
    /// $FF01
    pub serial_transfer_data: u8,
    pub serial_transfer_control: u8,
    _padding0: u8,
    /// $FF04
    pub timer_divider: u8,
    /// $FF05
    pub timer_tima: u8,
    /// $FF06
    pub timer_tma: u8,
    /// $FF07
    pub timer_tac: u8,
    _padding1: [u8; 7],
    /// $FF0F
    pub interrupts: u8,
    /// $FF10-$FF25
    pub audio: [u8; 22],
    _padding2: [u8; 10],
    /// $FF30-$FF3F
    pub wave_pattern: [u8; 16],
    /// $FF40
    pub lcd_control: LCDControl,
    /// $FF41
    pub lcd_stat: LCDStatus,
    /// $FF42
    pub lcd_scx: u8,
    /// $FF43
    pub lcd_scy: u8,
    /// $FF44
    pub lcd_y: u8,
    /// $FF45
    pub lcd_yc: u8,
    /// $FF46
    pub oam_dma_source_address: u8,
    /// $FF47
    pub bg_palette_data: u8,
    /// $FF48
    pub object_palette_0: u8,
    /// $FF49
    pub object_palette_1: u8,
    /// $FF4A
    pub wx: u8,
    /// $FF4B
    pub wy: u8,
    _padding3: [u8; 3],
    /// $FF4F
    pub vram_bank_select: u8,
    /// $FF50
    pub bootrom_disable: u8,
    /// $FF51-$FF55
    pub vram_dma: [u8; 5],
    _padding4: [u8; 18],
    /// $FF68-$FF6B
    pub bg_object_pallets: [u8; 4],
    _padding5: [u8; 4],
    /// $FF70
    pub wram_bank_select: u8,
    _padding6: [u8; 15],
    /// $FF80-$FFFE
    pub hram: [u8; 0x7f],
    /// $FFFF
    pub interrupt_enable: u8,
}

#[cfg(test)]
mod tests {
    use super::{IORegisters, LCDControl, LCDStatus, PPUMode};

    #[test]
    fn register_types_preserve_byte_layout() {
        assert_eq!(std::mem::size_of::<LCDControl>(), 1);
        assert_eq!(std::mem::align_of::<LCDControl>(), 1);
        assert_eq!(std::mem::size_of::<LCDStatus>(), 1);
        assert_eq!(std::mem::align_of::<LCDStatus>(), 1);
        assert_eq!(std::mem::size_of::<IORegisters>(), 0x200);
        assert_eq!(std::mem::align_of::<IORegisters>(), 1);
        assert_eq!(std::mem::offset_of!(IORegisters, lcd_control), 0x140);
        assert_eq!(std::mem::offset_of!(IORegisters, lcd_stat), 0x141);
    }

    #[test]
    fn lcd_control_exposes_named_bits() {
        let control = LCDControl::from_bits(0b1000_0110);

        assert!(control.obj_enable());
        assert!(control.obj_double_height());
        assert!(!control.window_enable());
        assert!(control.lcd_ppu_enable());
    }

    #[test]
    fn lcd_status_updates_mode_without_changing_other_bits() {
        let mut status = LCDStatus(0b1111_1100);

        status.set_ppu_mode(PPUMode::Drawing);

        assert_eq!(status.bits(), 0b1111_1111);
        assert_eq!(status.ppu_mode(), PPUMode::Drawing);
    }
}
