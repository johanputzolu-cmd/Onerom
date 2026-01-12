// Copyright (C) 2026 Piers Finlayson <piers@piers.rocks>
//
// MIT License

//! Contains Firmware Config objects

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// Top level configuration structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct FirmwareConfig {
    /// Optional MCU clock configuration
    pub ice_clock: Option<IceClockConfig>,

    /// Optional MCU clock configuration for Fire boards
    pub fire_clock: Option<FireClockConfig>,

    /// Optional LED configuration
    pub led: Option<LedConfig>,

    /// Optional Debug configuration
    pub swd: Option<DebugConfig>,

    /// Optional serving algorithm parameters
    pub serve_alg_params: Option<ServeAlgParams>,
}

impl FirmwareConfig {
    /// Deserialize 64-byte onerom_firmware_overrides_t structure into FirmwareConfig
    pub fn from_bytes(buf: &[u8]) -> Result<Self, String> {
        if buf.len() < 64 {
            return Err(format!("Buffer too small: {} bytes", buf.len()));
        }

        let mut offset = 0;

        // Read override_present (8 bytes)
        let override_present = &buf[offset..offset + 8];
        offset += 8;

        // Read frequencies (2 bytes each as u16)
        let ice_freq = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        offset += 2;
        let fire_freq = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        offset += 2;

        // Read fire_vreq (1 byte)
        let fire_vreq = buf[offset];
        offset += 1;

        // Skip pad1 (3 bytes)
        offset += 3;

        // Read override_value (8 bytes)
        let override_value = &buf[offset..offset + 8];
        // offset += 8; // Rest is padding

        // Reconstruct FirmwareConfig
        let ice_clock = if (override_present[0] & (1 << 0)) != 0 {
            Some(IceClockConfig {
                cpu_freq: ice_freq.try_into()
                    .map_err(|_| format!("Invalid ice_freq: {}", ice_freq))?,
                overclock: (override_value[0] & (1 << 0)) != 0,
            })
        } else {
            None
        };

        let fire_clock = if (override_present[0] & (1 << 2)) != 0 {
            Some(FireClockConfig {
                cpu_freq: fire_freq.try_into()
                    .map_err(|_| format!("Invalid fire_freq: {}", fire_freq))?,
                overclock: (override_value[0] & (1 << 1)) != 0,
                vreg: fire_vreq.try_into()
                    .map_err(|_| format!("Invalid fire_vreq: {}", fire_vreq))?,
            })
        } else {
            None
        };

        let led = if (override_present[0] & (1 << 5)) != 0 {
            Some(LedConfig {
                enabled: (override_value[0] & (1 << 2)) != 0,
            })
        } else {
            None
        };

        let swd = if (override_present[0] & (1 << 6)) != 0 {
            Some(DebugConfig {
                swd_enabled: (override_value[0] & (1 << 3)) != 0,
            })
        } else {
            None
        };

        Ok(FirmwareConfig {
            ice_clock,
            fire_clock,
            led,
            swd,
            serve_alg_params: None, // Stored separately
        })
    }
}

/// Ice Clock configuration structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct IceClockConfig {
    /// CPU frequency.  Only specific frequencies are supported
    pub cpu_freq: IceCpuFreq,

    /// Whether overclocking is enabled
    #[serde(default)]
    pub overclock: bool,
}

/// Ice Clock configuration structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct FireClockConfig {
    /// CPU frequency.  Only specific frequencies are supported
    pub cpu_freq: FireCpuFreq,

    /// Whether overclocking is enabled
    #[serde(default)]
    pub overclock: bool,

    /// Optional Vreg output voltage setting for RP2350 MCUs.
    #[serde(default)]
    pub vreg: FireVreg,
}

/// LED configuration structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct LedConfig {
    /// Whether the status LED is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Debug configuration structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct DebugConfig {
    /// Whether SWD debug interface is enabled
    #[serde(default = "default_true")]
    pub swd_enabled: bool,
}

/// Custom serving algorithm parameters
/// 
/// This is stored as unstructured parameters to allow for easy future
/// extension without breaking compatibility.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ServeAlgParams {
    pub params: Vec<u8>,
}

#[repr(u16)]
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum IceCpuFreq {
    #[serde(rename = "1MHz")]
    Mhz1 = 1,
    #[serde(rename = "2MHz")]
    Mhz2 = 2,
    #[serde(rename = "3MHz")]
    Mhz3 = 3,
    #[serde(rename = "4MHz")]
    Mhz4 = 4,
    #[serde(rename = "5MHz")]
    Mhz5 = 5,
    #[serde(rename = "6MHz")]
    Mhz6 = 6,
    #[serde(rename = "7MHz")]
    Mhz7 = 7,
    #[serde(rename = "8MHz")]
    Mhz8 = 8,
    #[serde(rename = "9MHz")]
    Mhz9 = 9,
    #[serde(rename = "10MHz")]
    Mhz10 = 10,
    #[serde(rename = "11MHz")]
    Mhz11 = 11,
    #[serde(rename = "12MHz")]
    Mhz12 = 12,
    #[serde(rename = "13MHz")]
    Mhz13 = 13,
    #[serde(rename = "14MHz")]
    Mhz14 = 14,
    #[serde(rename = "15MHz")]
    Mhz15 = 15,
    #[serde(rename = "16MHz")]
    Mhz16 = 16,
    #[serde(rename = "17MHz")]
    Mhz17 = 17,
    #[serde(rename = "18MHz")]
    Mhz18 = 18,
    #[serde(rename = "19MHz")]
    Mhz19 = 19,
    #[serde(rename = "20MHz")]
    Mhz20 = 20,
    #[serde(rename = "21MHz")]
    Mhz21 = 21,
    #[serde(rename = "22MHz")]
    Mhz22 = 22,
    #[serde(rename = "23MHz")]
    Mhz23 = 23,
    #[serde(rename = "24MHz")]
    Mhz24 = 24,
    #[serde(rename = "25MHz")]
    Mhz25 = 25,
    #[serde(rename = "26MHz")]
    Mhz26 = 26,
    #[serde(rename = "27MHz")]
    Mhz27 = 27,
    #[serde(rename = "28MHz")]
    Mhz28 = 28,
    #[serde(rename = "29MHz")]
    Mhz29 = 29,
    #[serde(rename = "30MHz")]
    Mhz30 = 30,
    #[serde(rename = "31MHz")]
    Mhz31 = 31,
    #[serde(rename = "32MHz")]
    Mhz32 = 32,
    #[serde(rename = "33MHz")]
    Mhz33 = 33,
    #[serde(rename = "34MHz")]
    Mhz34 = 34,
    #[serde(rename = "35MHz")]
    Mhz35 = 35,
    #[serde(rename = "36MHz")]
    Mhz36 = 36,
    #[serde(rename = "37MHz")]
    Mhz37 = 37,
    #[serde(rename = "38MHz")]
    Mhz38 = 38,
    #[serde(rename = "39MHz")]
    Mhz39 = 39,
    #[serde(rename = "40MHz")]
    Mhz40 = 40,
    #[serde(rename = "41MHz")]
    Mhz41 = 41,
    #[serde(rename = "42MHz")]
    Mhz42 = 42,
    #[serde(rename = "43MHz")]
    Mhz43 = 43,
    #[serde(rename = "44MHz")]
    Mhz44 = 44,
    #[serde(rename = "45MHz")]
    Mhz45 = 45,
    #[serde(rename = "46MHz")]
    Mhz46 = 46,
    #[serde(rename = "47MHz")]
    Mhz47 = 47,
    #[serde(rename = "48MHz")]
    Mhz48 = 48,
    #[serde(rename = "49MHz")]
    Mhz49 = 49,
    #[serde(rename = "50MHz")]
    Mhz50 = 50,
    #[serde(rename = "51MHz")]
    Mhz51 = 51,
    #[serde(rename = "52MHz")]
    Mhz52 = 52,
    #[serde(rename = "53MHz")]
    Mhz53 = 53,
    #[serde(rename = "54MHz")]
    Mhz54 = 54,
    #[serde(rename = "55MHz")]
    Mhz55 = 55,
    #[serde(rename = "56MHz")]
    Mhz56 = 56,
    #[serde(rename = "57MHz")]
    Mhz57 = 57,
    #[serde(rename = "58MHz")]
    Mhz58 = 58,
    #[serde(rename = "59MHz")]
    Mhz59 = 59,
    #[serde(rename = "60MHz")]
    Mhz60 = 60,
    #[serde(rename = "61MHz")]
    Mhz61 = 61,
    #[serde(rename = "62MHz")]
    Mhz62 = 62,
    #[serde(rename = "63MHz")]
    Mhz63 = 63,
    #[serde(rename = "64MHz")]
    Mhz64 = 64,
    #[serde(rename = "65MHz")]
    Mhz65 = 65,
    #[serde(rename = "66MHz")]
    Mhz66 = 66,
    #[serde(rename = "67MHz")]
    Mhz67 = 67,
    #[serde(rename = "68MHz")]
    Mhz68 = 68,
    #[serde(rename = "69MHz")]
    Mhz69 = 69,
    #[serde(rename = "70MHz")]
    Mhz70 = 70,
    #[serde(rename = "71MHz")]
    Mhz71 = 71,
    #[serde(rename = "72MHz")]
    Mhz72 = 72,
    #[serde(rename = "73MHz")]
    Mhz73 = 73,
    #[serde(rename = "74MHz")]
    Mhz74 = 74,
    #[serde(rename = "75MHz")]
    Mhz75 = 75,
    #[serde(rename = "76MHz")]
    Mhz76 = 76,
    #[serde(rename = "77MHz")]
    Mhz77 = 77,
    #[serde(rename = "78MHz")]
    Mhz78 = 78,
    #[serde(rename = "79MHz")]
    Mhz79 = 79,
    #[serde(rename = "80MHz")]
    Mhz80 = 80,
    #[serde(rename = "81MHz")]
    Mhz81 = 81,
    #[serde(rename = "82MHz")]
    Mhz82 = 82,
    #[serde(rename = "83MHz")]
    Mhz83 = 83,
    #[serde(rename = "84MHz")]
    Mhz84 = 84,
    #[serde(rename = "85MHz")]
    Mhz85 = 85,
    #[serde(rename = "86MHz")]
    Mhz86 = 86,
    #[serde(rename = "87MHz")]
    Mhz87 = 87,
    #[serde(rename = "88MHz")]
    Mhz88 = 88,
    #[serde(rename = "89MHz")]
    Mhz89 = 89,
    #[serde(rename = "90MHz")]
    Mhz90 = 90,
    #[serde(rename = "91MHz")]
    Mhz91 = 91,
    #[serde(rename = "92MHz")]
    Mhz92 = 92,
    #[serde(rename = "93MHz")]
    Mhz93 = 93,
    #[serde(rename = "94MHz")]
    Mhz94 = 94,
    #[serde(rename = "95MHz")]
    Mhz95 = 95,
    #[serde(rename = "96MHz")]
    Mhz96 = 96,
    #[serde(rename = "97MHz")]
    Mhz97 = 97,
    #[serde(rename = "98MHz")]
    Mhz98 = 98,
    #[serde(rename = "99MHz")]
    Mhz99 = 99,
    #[serde(rename = "100MHz")]
    Mhz100 = 100,
    #[serde(rename = "101MHz")]
    Mhz101 = 101,
    #[serde(rename = "102MHz")]
    Mhz102 = 102,
    #[serde(rename = "103MHz")]
    Mhz103 = 103,
    #[serde(rename = "104MHz")]
    Mhz104 = 104,
    #[serde(rename = "105MHz")]
    Mhz105 = 105,
    #[serde(rename = "106MHz")]
    Mhz106 = 106,
    #[serde(rename = "107MHz")]
    Mhz107 = 107,
    #[serde(rename = "108MHz")]
    Mhz108 = 108,
    #[serde(rename = "109MHz")]
    Mhz109 = 109,
    #[serde(rename = "110MHz")]
    Mhz110 = 110,
    #[serde(rename = "111MHz")]
    Mhz111 = 111,
    #[serde(rename = "112MHz")]
    Mhz112 = 112,
    #[serde(rename = "113MHz")]
    Mhz113 = 113,
    #[serde(rename = "114MHz")]
    Mhz114 = 114,
    #[serde(rename = "115MHz")]
    Mhz115 = 115,
    #[serde(rename = "116MHz")]
    Mhz116 = 116,
    #[serde(rename = "117MHz")]
    Mhz117 = 117,
    #[serde(rename = "118MHz")]
    Mhz118 = 118,
    #[serde(rename = "119MHz")]
    Mhz119 = 119,
    #[serde(rename = "120MHz")]
    Mhz120 = 120,
    #[serde(rename = "121MHz")]
    Mhz121 = 121,
    #[serde(rename = "122MHz")]
    Mhz122 = 122,
    #[serde(rename = "123MHz")]
    Mhz123 = 123,
    #[serde(rename = "124MHz")]
    Mhz124 = 124,
    #[serde(rename = "125MHz")]
    Mhz125 = 125,
    #[serde(rename = "126MHz")]
    Mhz126 = 126,
    #[serde(rename = "127MHz")]
    Mhz127 = 127,
    #[serde(rename = "128MHz")]
    Mhz128 = 128,
    #[serde(rename = "129MHz")]
    Mhz129 = 129,
    #[serde(rename = "130MHz")]
    Mhz130 = 130,
    #[serde(rename = "131MHz")]
    Mhz131 = 131,
    #[serde(rename = "132MHz")]
    Mhz132 = 132,
    #[serde(rename = "133MHz")]
    Mhz133 = 133,
    #[serde(rename = "134MHz")]
    Mhz134 = 134,
    #[serde(rename = "135MHz")]
    Mhz135 = 135,
    #[serde(rename = "136MHz")]
    Mhz136 = 136,
    #[serde(rename = "137MHz")]
    Mhz137 = 137,
    #[serde(rename = "138MHz")]
    Mhz138 = 138,
    #[serde(rename = "139MHz")]
    Mhz139 = 139,
    #[serde(rename = "140MHz")]
    Mhz140 = 140,
    #[serde(rename = "141MHz")]
    Mhz141 = 141,
    #[serde(rename = "142MHz")]
    Mhz142 = 142,
    #[serde(rename = "143MHz")]
    Mhz143 = 143,
    #[serde(rename = "144MHz")]
    Mhz144 = 144,
    #[serde(rename = "145MHz")]
    Mhz145 = 145,
    #[serde(rename = "146MHz")]
    Mhz146 = 146,
    #[serde(rename = "147MHz")]
    Mhz147 = 147,
    #[serde(rename = "148MHz")]
    Mhz148 = 148,
    #[serde(rename = "149MHz")]
    Mhz149 = 149,
    #[serde(rename = "150MHz")]
    Mhz150 = 150,
    #[serde(rename = "151MHz")]
    Mhz151 = 151,
    #[serde(rename = "152MHz")]
    Mhz152 = 152,
    #[serde(rename = "153MHz")]
    Mhz153 = 153,
    #[serde(rename = "154MHz")]
    Mhz154 = 154,
    #[serde(rename = "155MHz")]
    Mhz155 = 155,
    #[serde(rename = "156MHz")]
    Mhz156 = 156,
    #[serde(rename = "157MHz")]
    Mhz157 = 157,
    #[serde(rename = "158MHz")]
    Mhz158 = 158,
    #[serde(rename = "159MHz")]
    Mhz159 = 159,
    #[serde(rename = "160MHz")]
    Mhz160 = 160,
    #[serde(rename = "161MHz")]
    Mhz161 = 161,
    #[serde(rename = "162MHz")]
    Mhz162 = 162,
    #[serde(rename = "163MHz")]
    Mhz163 = 163,
    #[serde(rename = "164MHz")]
    Mhz164 = 164,
    #[serde(rename = "165MHz")]
    Mhz165 = 165,
    #[serde(rename = "166MHz")]
    Mhz166 = 166,
    #[serde(rename = "167MHz")]
    Mhz167 = 167,
    #[serde(rename = "168MHz")]
    Mhz168 = 168,
    #[serde(rename = "169MHz")]
    Mhz169 = 169,
    #[serde(rename = "170MHz")]
    Mhz170 = 170,
    #[serde(rename = "171MHz")]
    Mhz171 = 171,
    #[serde(rename = "172MHz")]
    Mhz172 = 172,
    #[serde(rename = "173MHz")]
    Mhz173 = 173,
    #[serde(rename = "174MHz")]
    Mhz174 = 174,
    #[serde(rename = "175MHz")]
    Mhz175 = 175,
    #[serde(rename = "176MHz")]
    Mhz176 = 176,
    #[serde(rename = "177MHz")]
    Mhz177 = 177,
    #[serde(rename = "178MHz")]
    Mhz178 = 178,
    #[serde(rename = "179MHz")]
    Mhz179 = 179,
    #[serde(rename = "180MHz")]
    Mhz180 = 180,
    #[default]
    Stock = 0xFFFF,
}

// Macro for IceCpuFreq since it's just 1-180
macro_rules! impl_ice_tryfrom {
    ($($num:literal => $variant:ident),* $(,)?) => {
        impl TryFrom<u16> for IceCpuFreq {
            type Error = u16;
            
            fn try_from(value: u16) -> Result<Self, Self::Error> {
                match value {
                    $($num => Ok(Self::$variant),)*
                    0xFFFF => Ok(Self::Stock),
                    _ => Err(value),
                }
            }
        }
    };
}

impl_ice_tryfrom! {
    1 => Mhz1, 2 => Mhz2, 3 => Mhz3, 4 => Mhz4, 5 => Mhz5,
    6 => Mhz6, 7 => Mhz7, 8 => Mhz8, 9 => Mhz9, 10 => Mhz10,
    11 => Mhz11, 12 => Mhz12, 13 => Mhz13, 14 => Mhz14, 15 => Mhz15,
    16 => Mhz16, 17 => Mhz17, 18 => Mhz18, 19 => Mhz19, 20 => Mhz20,
    21 => Mhz21, 22 => Mhz22, 23 => Mhz23, 24 => Mhz24, 25 => Mhz25,
    26 => Mhz26, 27 => Mhz27, 28 => Mhz28, 29 => Mhz29, 30 => Mhz30,
    31 => Mhz31, 32 => Mhz32, 33 => Mhz33, 34 => Mhz34, 35 => Mhz35,
    36 => Mhz36, 37 => Mhz37, 38 => Mhz38, 39 => Mhz39, 40 => Mhz40,
    41 => Mhz41, 42 => Mhz42, 43 => Mhz43, 44 => Mhz44, 45 => Mhz45,
    46 => Mhz46, 47 => Mhz47, 48 => Mhz48, 49 => Mhz49, 50 => Mhz50,
    51 => Mhz51, 52 => Mhz52, 53 => Mhz53, 54 => Mhz54, 55 => Mhz55,
    56 => Mhz56, 57 => Mhz57, 58 => Mhz58, 59 => Mhz59, 60 => Mhz60,
    61 => Mhz61, 62 => Mhz62, 63 => Mhz63, 64 => Mhz64, 65 => Mhz65,
    66 => Mhz66, 67 => Mhz67, 68 => Mhz68, 69 => Mhz69, 70 => Mhz70,
    71 => Mhz71, 72 => Mhz72, 73 => Mhz73, 74 => Mhz74, 75 => Mhz75,
    76 => Mhz76, 77 => Mhz77, 78 => Mhz78, 79 => Mhz79, 80 => Mhz80,
    81 => Mhz81, 82 => Mhz82, 83 => Mhz83, 84 => Mhz84, 85 => Mhz85,
    86 => Mhz86, 87 => Mhz87, 88 => Mhz88, 89 => Mhz89, 90 => Mhz90,
    91 => Mhz91, 92 => Mhz92, 93 => Mhz93, 94 => Mhz94, 95 => Mhz95,
    96 => Mhz96, 97 => Mhz97, 98 => Mhz98, 99 => Mhz99, 100 => Mhz100,
    101 => Mhz101, 102 => Mhz102, 103 => Mhz103, 104 => Mhz104, 105 => Mhz105,
    106 => Mhz106, 107 => Mhz107, 108 => Mhz108, 109 => Mhz109, 110 => Mhz110,
    111 => Mhz111, 112 => Mhz112, 113 => Mhz113, 114 => Mhz114, 115 => Mhz115,
    116 => Mhz116, 117 => Mhz117, 118 => Mhz118, 119 => Mhz119, 120 => Mhz120,
    121 => Mhz121, 122 => Mhz122, 123 => Mhz123, 124 => Mhz124, 125 => Mhz125,
    126 => Mhz126, 127 => Mhz127, 128 => Mhz128, 129 => Mhz129, 130 => Mhz130,
    131 => Mhz131, 132 => Mhz132, 133 => Mhz133, 134 => Mhz134, 135 => Mhz135,
    136 => Mhz136, 137 => Mhz137, 138 => Mhz138, 139 => Mhz139, 140 => Mhz140,
    141 => Mhz141, 142 => Mhz142, 143 => Mhz143, 144 => Mhz144, 145 => Mhz145,
    146 => Mhz146, 147 => Mhz147, 148 => Mhz148, 149 => Mhz149, 150 => Mhz150,
    151 => Mhz151, 152 => Mhz152, 153 => Mhz153, 154 => Mhz154, 155 => Mhz155,
    156 => Mhz156, 157 => Mhz157, 158 => Mhz158, 159 => Mhz159, 160 => Mhz160,
    161 => Mhz161, 162 => Mhz162, 163 => Mhz163, 164 => Mhz164, 165 => Mhz165,
    166 => Mhz166, 167 => Mhz167, 168 => Mhz168, 169 => Mhz169, 170 => Mhz170,
    171 => Mhz171, 172 => Mhz172, 173 => Mhz173, 174 => Mhz174, 175 => Mhz175,
    176 => Mhz176, 177 => Mhz177, 178 => Mhz178, 179 => Mhz179, 180 => Mhz180,
}

#[repr(u16)]
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum FireCpuFreq {
    #[serde(rename = "20MHz")]
    MHz20 = 1,
    #[serde(rename = "30MHz")]
    MHz30 = 2,
    #[serde(rename = "40MHz")]
    MHz40 = 3,
    #[serde(rename = "50MHz")]
    MHz50 = 4,
    #[serde(rename = "60MHz")]
    MHz60 = 5,
    #[serde(rename = "70MHz")]
    MHz70 = 6,
    #[serde(rename = "80MHz")]
    MHz80 = 7,
    #[serde(rename = "90MHz")]
    MHz90 = 8,
    #[serde(rename = "100MHz")]
    MHz100 = 9,
    #[serde(rename = "110MHz")]
    MHz110 = 10,
    #[serde(rename = "120MHz")]
    MHz120 = 11,
    #[serde(rename = "130MHz")]
    MHz130 = 12,
    #[serde(rename = "140MHz")]
    MHz140 = 13,
    #[serde(rename = "150MHz")]
    MHz150 = 14,
    #[serde(rename = "160MHz")]
    MHz160 = 15,
    #[serde(rename = "170MHz")]
    MHz170 = 16,
    #[serde(rename = "180MHz")]
    MHz180 = 17,
    #[serde(rename = "190MHz")]
    MHz190 = 18,
    #[serde(rename = "200MHz")]
    MHz200 = 19,
    #[serde(rename = "210MHz")]
    MHz210 = 20,
    #[serde(rename = "220MHz")]
    MHz220 = 21,
    #[serde(rename = "230MHz")]
    MHz230 = 22,
    #[serde(rename = "240MHz")]
    MHz240 = 23,
    #[serde(rename = "250MHz")]
    MHz250 = 24,
    #[serde(rename = "260MHz")]
    MHz260 = 25,
    #[serde(rename = "270MHz")]
    MHz270 = 26,
    #[serde(rename = "280MHz")]
    MHz280 = 27,
    #[serde(rename = "300MHz")]
    MHz300 = 28,
    #[serde(rename = "320MHz")]
    MHz320 = 29,
    #[serde(rename = "330MHz")]
    MHz330 = 30,
    #[serde(rename = "340MHz")]
    MHz340 = 31,
    #[serde(rename = "360MHz")]
    MHz360 = 32,
    #[serde(rename = "380MHz")]
    MHz380 = 33,
    #[serde(rename = "390MHz")]
    MHz390 = 34,
    #[serde(rename = "400MHz")]
    MHz400 = 35,
    #[serde(rename = "420MHz")]
    MHz420 = 36,
    #[serde(rename = "440MHz")]
    MHz440 = 37,
    #[serde(rename = "450MHz")]
    MHz450 = 38,
    #[serde(rename = "460MHz")]
    MHz460 = 39,
    #[serde(rename = "480MHz")]
    MHz480 = 40,
    #[serde(rename = "500MHz")]
    MHz500 = 41,
    #[serde(rename = "510MHz")]
    MHz510 = 42,
    #[serde(rename = "520MHz")]
    MHz520 = 43,
    #[serde(rename = "540MHz")]
    MHz540 = 44,
    #[default]
    Stock = 0xFFFF,
}

impl TryFrom<u16> for FireCpuFreq {
    type Error = u16;
    
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::MHz20),
            2 => Ok(Self::MHz30),
            3 => Ok(Self::MHz40),
            4 => Ok(Self::MHz50),
            5 => Ok(Self::MHz60),
            6 => Ok(Self::MHz70),
            7 => Ok(Self::MHz80),
            8 => Ok(Self::MHz90),
            9 => Ok(Self::MHz100),
            10 => Ok(Self::MHz110),
            11 => Ok(Self::MHz120),
            12 => Ok(Self::MHz130),
            13 => Ok(Self::MHz140),
            14 => Ok(Self::MHz150),
            15 => Ok(Self::MHz160),
            16 => Ok(Self::MHz170),
            17 => Ok(Self::MHz180),
            18 => Ok(Self::MHz190),
            19 => Ok(Self::MHz200),
            20 => Ok(Self::MHz210),
            21 => Ok(Self::MHz220),
            22 => Ok(Self::MHz230),
            23 => Ok(Self::MHz240),
            24 => Ok(Self::MHz250),
            25 => Ok(Self::MHz260),
            26 => Ok(Self::MHz270),
            27 => Ok(Self::MHz280),
            28 => Ok(Self::MHz300),
            29 => Ok(Self::MHz320),
            30 => Ok(Self::MHz330),
            31 => Ok(Self::MHz340),
            32 => Ok(Self::MHz360),
            33 => Ok(Self::MHz380),
            34 => Ok(Self::MHz390),
            35 => Ok(Self::MHz400),
            36 => Ok(Self::MHz420),
            37 => Ok(Self::MHz440),
            38 => Ok(Self::MHz450),
            39 => Ok(Self::MHz460),
            40 => Ok(Self::MHz480),
            41 => Ok(Self::MHz500),
            42 => Ok(Self::MHz510),
            43 => Ok(Self::MHz520),
            44 => Ok(Self::MHz540),
            0xFFFF => Ok(Self::Stock),
            _ => Err(value),
        }
    }
}

/// Voltage regulator setting for RP2350 MCUs
#[repr(u8)]
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum FireVreg {
    #[serde(rename = "0.55V")]
    V0_55 = 0x00,
    #[serde(rename = "0.60V")]
    V0_60 = 0x01,
    #[serde(rename = "0.65V")]
    V0_65 = 0x02,
    #[serde(rename = "0.70V")]
    V0_70 = 0x03,
    #[serde(rename = "0.75V")]
    V0_75 = 0x04,
    #[serde(rename = "0.80V")]
    V0_80 = 0x05,
    #[serde(rename = "0.85V")]
    V0_85 = 0x06,
    #[serde(rename = "0.90V")]
    V0_90 = 0x07,
    #[serde(rename = "0.95V")]
    V0_95 = 0x08,
    #[serde(rename = "1.00V")]
    V1_00 = 0x09,
    #[serde(rename = "1.05V")]
    V1_05 = 0x0A,
    #[serde(rename = "1.10V")]
    V1_10 = 0x0B,
    #[serde(rename = "1.15V")]
    V1_15 = 0x0C,
    #[serde(rename = "1.20V")]
    V1_20 = 0x0D,
    #[serde(rename = "1.25V")]
    V1_25 = 0x0E,
    #[serde(rename = "1.30V")]
    V1_30 = 0x0F,
    #[serde(rename = "1.35V")]
    V1_35 = 0x10,
    #[serde(rename = "1.40V")]
    V1_40 = 0x11,
    #[serde(rename = "1.50V")]
    V1_50 = 0x12,
    #[serde(rename = "1.60V")]
    V1_60 = 0x13,
    #[serde(rename = "1.65V")]
    V1_65 = 0x14,
    #[serde(rename = "1.70V")]
    V1_70 = 0x15,
    #[serde(rename = "1.80V")]
    V1_80 = 0x16,
    #[serde(rename = "1.90V")]
    V1_90 = 0x17,
    #[serde(rename = "2.00V")]
    V2_00 = 0x18,
    #[serde(rename = "2.35V")]
    V2_35 = 0x19,
    #[serde(rename = "2.50V")]
    V2_50 = 0x1A,
    #[serde(rename = "2.65V")]
    V2_65 = 0x1B,
    #[serde(rename = "2.80V")]
    V2_80 = 0x1C,
    #[serde(rename = "3.00V")]
    V3_00 = 0x1D,
    #[serde(rename = "3.15V")]
    V3_15 = 0x1E,
    #[serde(rename = "3.30V")]
    V3_30 = 0x1F,
    #[default]
    Stock = 0xFF,
}

impl TryFrom<u8> for FireVreg {
    type Error = u8;
    
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::V0_55),
            0x01 => Ok(Self::V0_60),
            0x02 => Ok(Self::V0_65),
            0x03 => Ok(Self::V0_70),
            0x04 => Ok(Self::V0_75),
            0x05 => Ok(Self::V0_80),
            0x06 => Ok(Self::V0_85),
            0x07 => Ok(Self::V0_90),
            0x08 => Ok(Self::V0_95),
            0x09 => Ok(Self::V1_00),
            0x0A => Ok(Self::V1_05),
            0x0B => Ok(Self::V1_10),
            0x0C => Ok(Self::V1_15),
            0x0D => Ok(Self::V1_20),
            0x0E => Ok(Self::V1_25),
            0x0F => Ok(Self::V1_30),
            0x10 => Ok(Self::V1_35),
            0x11 => Ok(Self::V1_40),
            0x12 => Ok(Self::V1_50),
            0x13 => Ok(Self::V1_60),
            0x14 => Ok(Self::V1_65),
            0x15 => Ok(Self::V1_70),
            0x16 => Ok(Self::V1_80),
            0x17 => Ok(Self::V1_90),
            0x18 => Ok(Self::V2_00),
            0x19 => Ok(Self::V2_35),
            0x1A => Ok(Self::V2_50),
            0x1B => Ok(Self::V2_65),
            0x1C => Ok(Self::V2_80),
            0x1D => Ok(Self::V3_00),
            0x1E => Ok(Self::V3_15),
            0x1F => Ok(Self::V3_30),
            0xFF => Ok(Self::Stock),
            _ => Err(value),
        }
    }
}