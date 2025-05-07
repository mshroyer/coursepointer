/// Implements the Garmin FIT CRC algorithm.
///
/// A direct transcription of Garmin's reference implementation at
/// https://developer.garmin.com/fit/protocol/
struct Crc {
    crc: u16,
}

static CRC_TABLE: &'static [u16] = &[
    0x0000, 0xCC01, 0xD801, 0x1400, 0xF001, 0x3C00, 0x2800, 0xE401, 0xA001, 0x6C00, 0x7800, 0xB401,
    0x5000, 0x9C01, 0x8801, 0x4400,
];

impl Crc {
    pub fn new() -> Self {
        // Garmin's docs don't say so explicitly, but the starting value is zero.
        Self { crc: 0 }
    }

    pub fn add_byte(&mut self, byte: u8) {
        // Checksum lower four bits
        let mut tmp = CRC_TABLE[(self.crc & 0x0F) as usize];
        self.crc = (self.crc >> 4) & 0x0FFF;
        self.crc = self.crc ^ tmp ^ CRC_TABLE[(byte & 0x0F) as usize];

        // Checksum upper four bits
        tmp = CRC_TABLE[(self.crc & 0x0F) as usize];
        self.crc = (self.crc >> 4) & 0x0FFF;
        self.crc = self.crc ^ tmp ^ CRC_TABLE[(byte >> 4) as usize];
    }

    pub fn add_bytes(&mut self, byte: &[u8]) {
        for byte in byte {
            self.add_byte(*byte);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Crc;

    #[test]
    fn test_header_crc() {
        let mut crc = Crc::new();
        // A header from a FIT file I exported from Garmin Connect, minus its CRC bytes.
        crc.add_bytes(&[
            0x0e, 0x10, 0xb2, 0x52, 0x88, 0x42, 0x00, 0x00, 0x2e, 0x46, 0x49, 0x54,
        ]);
        // The CRC value from the last two bytes of the header, interpreted as little endian.
        assert_eq!(crc.crc, 0xf94b);
    }
}
